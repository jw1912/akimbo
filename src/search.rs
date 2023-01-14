use super::{consts::*, position::Position, tables::{Bound, HashTable, HistoryScore, KillerTable}, movegen::MoveList, u16_to_uci, from, to};
use std::{cmp::{min, max}, time::Instant};

/// Determines what is done in the node
struct Nt(bool, bool, bool);

/// Contains everything needed for a search.
pub struct Ctx {
    pub hash_table: HashTable,
    killer_table: KillerTable,
    history_table: [[[HistoryScore; 64]; 64]; 2],
    pub alloc_time: u128,
    time: Instant,
    nodes: u64,
    qnodes: u64,
    ply: i16,
    seldepth: i16,
    abort: bool,
}

impl Ctx {
    pub fn new() -> Self {
        Self {
            hash_table: HashTable::new(),
            killer_table: KillerTable([[0; KILLERS_PER_PLY]; MAX_PLY as usize + 1]),
            history_table: [[[HistoryScore::default(); 64]; 64]; 2],
            time: Instant::now(), alloc_time: 1000, nodes: 0, qnodes: 0, ply: 0, seldepth: 0, abort: false
        }
    }

    fn reset(&mut self) {
        self.time = Instant::now();
        self.nodes = 0;
        self.qnodes = 0;
        self.ply = 0;
        self.seldepth = 0;
        self.abort = false;
        self.killer_table = KillerTable([[0; KILLERS_PER_PLY]; MAX_PLY as usize + 1]);
        self.history_table = [[[HistoryScore::default(); 64]; 64]; 2];
    }

    fn timer(&self) -> u128 {
        self.time.elapsed().as_millis()
    }
}

impl Position {
    fn mvv_lva(&self, m: u16) -> u16 {
        let mpc: usize = self.squares[from!(m)] as usize;
        let cpc: usize = self.squares[to!(m)] as usize;
        MVV_LVA[cpc * usize::from(cpc != 6)][mpc]
    }

    fn score_move(&self, m: u16, hash_move: u16, killers: &[u16; KILLERS_PER_PLY], d: i8, ctx: &mut Ctx) -> u16 {
        if m == hash_move {
            HASH_MOVE
        } else if m & 0b0100_0000_0000_0000 > 0 {
            self.mvv_lva(m)
        } else if m & 0b1000_0000_0000_0000 > 0 {
            PROMOTION
        } else if killers.contains(&m) {
            KILLER
        } else {
            let entry: &mut HistoryScore = &mut ctx.history_table[usize::from(self.c)][from!(m)][to!(m)];
            entry.1 += (d as u64).pow(2);
            (800 * entry.0 / (entry.1 + 1)) as u16
        }
    }

    fn score(&self, moves: &MoveList, hash_move: u16, d: i8, ctx: &mut Ctx) -> MoveList {
        let mut scores: MoveList = MoveList::default();
        let killers: [u16; KILLERS_PER_PLY] = ctx.killer_table.0[ctx.ply as usize];
        for i in 0..moves.len { scores.push(self.score_move(moves.list[i], hash_move, &killers, d, ctx)) }
        scores
    }

    fn score_caps(&self, caps: &MoveList) -> MoveList {
        let mut scores: MoveList = MoveList::default();
        for i in 0..caps.len { scores.push(self.mvv_lva(caps.list[i])) }
        scores
    }
}

impl MoveList {
    // O(n^2) algorithm to incrementally sort the move list as needed.
    fn pick(&mut self, scores: &mut MoveList) -> Option<(u16, u16)> {
        if scores.len == 0 {return None}
        let mut idx: usize = 0;
        let mut best: u16 = 0;
        let mut score: u16;
        for i in 0..scores.len {
            score = scores.list[i];
            if score > best {
                best = score;
                idx = i;
            }
        }
        scores.len -= 1;
        scores.list.swap(idx, scores.len);
        self.list.swap(idx, scores.len);
        Some((self.list[scores.len], best))
    }
}

fn pvs(pos: &mut Position, mut a: i16, mut b: i16, mut d: i8, ctx: &mut Ctx, nt: Nt, line: &mut Vec<u16>) -> i16 {
    // search aborting
    if ctx.abort { return 0 }
    if ctx.nodes & 1023 == 0 && ctx.timer() >= ctx.alloc_time {
        ctx.abort = true;
        return 0
    }

    if pos.is_draw(ctx.ply) { return 0 }
    let Nt(pv, in_check, null): Nt = nt;

    // mate distance pruning
    a = max(a, -MAX + ctx.ply);
    b = min(b, MAX - ctx.ply - 1);
    if a >= b { return a }

    // check extensions
    d += i8::from(in_check);

    if d <= 0 || ctx.ply == MAX_PLY {
        ctx.seldepth = max(ctx.seldepth, ctx.ply);
        return quiesce(pos, a, b, &mut ctx.qnodes)
    }

    // probing hash table
    let mut bm: u16 = 0;
    let mut write: bool = true;
    if let Some(res) = ctx.hash_table.probe(pos.state.zobrist, ctx.ply) {
        write = d > res.depth;
        bm = res.best_move;
        if ctx.ply > 0 && res.depth >= d && pos.state.halfmove_clock < 90 && match res.bound {
            Bound::Lower => res.score >= b,
            Bound::Upper => res.score <= a,
            Bound::Exact => !pv, // want nice pv lines
        } { return res.score }
    }

    // pruning
    if !pv && !in_check && b.abs() < MATE {
        let eval: i16 = pos.eval();

        // reverse futility pruning
        let margin: i16 = eval - 120 * i16::from(d);
        if d <= 8 && margin >= b { return margin }

        // null move pruning
        if null && d >= 3 && pos.phase >= 6 && eval >= b {
            let copy: (u16, u64) = pos.do_null();
            let nw: i16 = -pvs(pos, -b, -b + 1, d - 3, ctx, Nt(false, false, false), &mut Vec::new());
            pos.undo_null(copy);
            if nw >= b {return nw}
            if nw.abs() >= MATE {d += 1}
        }
    }

    ctx.nodes += 1;
    ctx.ply += 1;
    let lmr: bool = d > 2 && ctx.ply > 1 && !in_check;
    let mut moves: MoveList = pos.gen::<ALL>();
    let mut scores: MoveList = pos.score(&moves, bm, d, ctx);
    let (mut legal, mut eval, mut bound, mut quiet): (u16, i16, Bound, u16) = (0, -MAX, Bound::Upper, 0);
    let (mut check, mut r, mut score, mut zw): (bool, i8, i16, i16);
    let mut sline: Vec<u16> = Vec::new();
    while let Some((m, ms)) = moves.pick(&mut scores) {
        if pos.r#do(m) { continue }
        legal += 1;
        check = pos.is_in_check();

        // late move reductions
        quiet += u16::from(ms < KILLER);
        r = i8::from(lmr && !check && legal > 1 && quiet > 0);

        // principle variation search
        sline.clear();
        score = if legal == 1 {
            -pvs(pos, -b, -a, d - 1, ctx, Nt(pv, check, false), &mut sline)
        } else {
            zw = -pvs(pos, -a - 1, -a, d - 1 - r, ctx, Nt(false, check, true), &mut sline);
            if (a != b - 1 || r > 0) && zw > a {
                -pvs(pos, -b, -a, d - 1, ctx, Nt(pv, check, false), &mut sline)
            } else { zw }
        };
        pos.undo();

        if score > eval {
            eval = score;
            bm = m;
            if score > a {
                a = score;
                bound = Bound::Exact;
                line.clear();
                line.push(m);
                line.append(&mut sline);
                if score >= b {
                    bound = Bound::Lower;
                    if ms <= KILLER {
                        ctx.killer_table.push(m, ctx.ply);
                        ctx.history_table[usize::from(pos.c)][from!(m)][to!(m)].0 += (d as u64).pow(2);
                    }
                    break
                }
            }
        }
    }
    ctx.ply -= 1;
    if legal == 0 { return i16::from(in_check) * (-MAX + ctx.ply) }
    if write && !ctx.abort { ctx.hash_table.push(pos.state.zobrist, bm, d, bound, eval, ctx.ply) }
    eval
}

fn quiesce(p: &mut Position, mut a: i16, b: i16, qn: &mut u64) -> i16 {
    *qn += 1;
    let mut e: i16 = p.eval();
    if e >= b { return e }
    a = max(a, e);
    let mut caps: MoveList = p.gen::<CAPTURES>();
    let mut scores: MoveList = p.score_caps(&caps);
    while let Some((m, _)) = caps.pick(&mut scores) {
        if p.r#do(m) {continue}
        let s: i16 = -quiesce(p, -b, -a, qn);
        p.undo();
        if s >= b { return s }
        e = max(e, s);
        a = max(a, s);
    }
    e
}

pub fn go(pos: &mut Position, allocated_depth: i8, ctx: &mut Ctx) {
    ctx.reset();
    let mut best_move: u16 = 0;
    let in_check: bool = pos.is_in_check();
    for d in 1..=allocated_depth {
        let mut pv_line: Vec<u16> = Vec::new();
        let score: i16 = pvs(pos, -MAX, MAX, d, ctx, Nt(true, in_check, false), &mut pv_line);
        if ctx.abort { break }
        best_move = pv_line[0];
        let (stype, sval): (&str, i16) = if score.abs() >= MATE {
            ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2)
        } else {("cp", score)};
        let t: u128 = ctx.timer();
        let nodes: u64 = ctx.nodes + ctx.qnodes;
        let nps: u32 = ((nodes as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = pv_line.iter().map(|m: &u16| u16_to_uci(pos, *m)).collect::<String>();
        println!("info depth {d} seldepth {} score {stype} {sval} time {t} nodes {nodes} nps {nps} pv {pv_str}", ctx.seldepth);
    }
    println!("bestmove {}", u16_to_uci(pos, best_move));
}
