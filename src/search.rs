use super::{consts::*, position::Position, tables::{Bound, HashTable, KillerTable}, movegen::MoveList, u16_to_uci, from, to};
use std::{cmp::{min, max}, time::Instant};

/// Determines what is done in the node
struct Nt(u8);
impl Nt {
    fn encode(pv: bool, check: bool, null: bool) -> Self {
        Self(4 * u8::from(pv) + 2 * u8::from(check) + u8::from(null))
    }
}

/// Contains everything needed for a search.
pub struct SearchContext {
    pub hash_table: HashTable,
    killer_table: KillerTable,
    pub alloc_time: u128,
    time: Instant,
    nodes: u64,
    qnodes: u64,
    ply: i16,
    seldepth: i16,
    abort: bool,
}

impl SearchContext{
    pub fn new(hash_table: HashTable, killer_table: KillerTable) -> Self {
        Self { hash_table, killer_table, time: Instant::now(), alloc_time: 1000, nodes: 0, qnodes: 0, ply: 0, seldepth: 0, abort: false}
    }

    fn reset(&mut self) {
        self.time = Instant::now();
        self.nodes = 0;
        self.qnodes = 0;
        self.ply = 0;
        self.seldepth = 0;
        self.abort = false;
        self.killer_table.clear();
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

    fn score_move(&self, m: u16, hash_move: u16, killers: &[u16; KILLERS_PER_PLY]) -> u16 {
        if m == hash_move {
            HASH_MOVE
        } else if m & 0b0100_0000_0000_0000 > 0 {
            self.mvv_lva(m)
        } else if m & 0b1000_0000_0000_0000 > 0 {
            PROMOTION
        } else if killers.contains(&m) {
            KILLER
        } else {
            0
        }
    }

    fn score(&self, moves: &MoveList, hash_move: u16, ply: i16, ctx: &SearchContext) -> MoveList {
        let mut scores: MoveList = MoveList::default();
        let killers: [u16; KILLERS_PER_PLY] = ctx.killer_table.0[ply as usize];
        for i in 0..moves.len { scores.push(self.score_move(moves.list[i], hash_move, &killers)) }
        scores
    }

    fn score_captures(&self, moves: &MoveList) -> MoveList {
        let mut scores: MoveList = MoveList::default();
        for i in 0..moves.len { scores.push(self.mvv_lva(moves.list[i])) }
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

fn pvs(pos: &mut Position, nt: Nt, mut a: i16, mut b: i16, mut d: i8, ctx: &mut SearchContext, line: &mut Vec<u16>) -> i16 {
    // search aborting
    if ctx.abort { return 0 }
    if ctx.nodes & 1023 == 0 && ctx.timer() >= ctx.alloc_time {
        ctx.abort = true;
        return 0
    }

    if pos.is_draw(ctx.ply) { return 0 }
    let (pv, in_check, null): (bool, bool, bool) = (nt.0 & 4 > 0, nt.0 & 2 > 0, nt.0 & 1 > 0);

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
    if !pv && !in_check && b.abs() < MATE_THRESHOLD {
        let mut eval: i16 = pos.eval();

        // reverse futility pruning
        let margin: i16 = eval - 120 * i16::from(d);
        if d <= 8 && margin >= b { return margin }

        // null move pruning
        if null && d >= 3 && pos.phase >= 6 && eval >= b {
            let copy: (u16, u64) = pos.do_null();
            let r: i8 = 3 + i8::from(d > 8);
            eval = -pvs(pos, Nt::encode(false, false, false), -b, -b + 1, d - r, ctx, &mut Vec::new());
            pos.undo_null(copy);
            if eval >= b {return eval}
        }
    }

    ctx.nodes += 1;
    ctx.ply += 1;
    let lmr: bool = d >= 2 && ctx.ply > 0 && !in_check;
    let mut moves: MoveList = pos.gen::<ALL>();
    let mut scores: MoveList = pos.score(&moves, bm, ctx.ply, ctx);
    let (mut legal, mut eval, mut bound): (u16, i16, Bound) = (0, -MAX, Bound::Upper);
    while let Some((m, ms)) = moves.pick(&mut scores) {
        if pos.do_move(m) { continue }
        legal += 1;
        let check: bool = pos.is_in_check();

        // late move reductions
        let r: i8 = i8::from(lmr && !check && legal > 2 && ms < KILLER);

        // principle variation search
        let mut sline: Vec<u16> = Vec::new();
        let score: i16 = if legal == 1 {
            -pvs(pos, Nt::encode(pv, check, false), -b, -a, d - 1, ctx, &mut sline)
        } else {
            let zw_score: i16 = -pvs(pos, Nt::encode(false, check, true), -a - 1, -a, d - 1 - r, ctx, &mut sline);
            if (a != b - 1 || r > 0) && zw_score > a {
                -pvs(pos, Nt::encode(pv, check, false), -b, -a, d - 1, ctx, &mut sline)
            } else { zw_score }
        };
        pos.undo_move();

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
                    if ms <= KILLER {ctx.killer_table.push(m, ctx.ply)}
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

fn quiesce(pos: &mut Position, mut a: i16, b: i16, qnodes: &mut u64) -> i16 {
    *qnodes += 1;
    let mut eval: i16 = pos.eval();
    if eval >= b { return eval }
    a = max(a, eval);
    let mut captures: MoveList = pos.gen::<CAPTURES>();
    let mut scores: MoveList = pos.score_captures(&captures);
    while let Some((m, _)) = captures.pick(&mut scores) {
        if pos.do_move(m) {continue}
        let score: i16 = -quiesce(pos, -b, -a, qnodes);
        pos.undo_move();
        if score >= b { return score }
        eval = max(eval, score);
        a = max(a, score);
    }
    eval
}

pub fn go(pos: &mut Position, allocated_depth: i8, ctx: &mut SearchContext) {
    ctx.reset();
    let mut best_move: u16 = 0;
    let in_check: bool = pos.is_in_check();
    for d in 1..=allocated_depth {
        let mut pv_line: Vec<u16> = Vec::new();
        let score: i16 = pvs(pos, Nt::encode(true, in_check, false), -MAX, MAX, d, ctx, &mut pv_line);
        if ctx.abort { break }
        best_move = pv_line[0];
        let (stype, sval): (&str, i16) = if score.abs() >= MATE_THRESHOLD {
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
