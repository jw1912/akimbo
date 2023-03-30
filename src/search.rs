use super::{consts::*, position::Pos, tables::*, movegen::MoveList, u16_to_uci, from, to};
use std::{cmp::{min, max}, time::Instant};

/// Determines what is done in the node
struct Nt(bool, bool);

/// Contains everything needed for a search.
pub struct Ctx {
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

impl Ctx {
    pub fn new() -> Self {
        Self {
            hash_table: HashTable::new(),
            killer_table: KillerTable([[0; KILLERS]; MAX_PLY as usize + 1]),
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
        self.killer_table = KillerTable([[0; KILLERS]; MAX_PLY as usize + 1]);
    }

    fn timer(&self) -> u128 {
        self.time.elapsed().as_millis()
    }
}

// move scoring
impl Pos {
    fn score(&self, moves: &MoveList, hash_move: u16, ctx: &Ctx) -> MoveList {
        let mut scores = MoveList::uninit();
        let killers: [u16; KILLERS] = ctx.killer_table.0[ctx.ply as usize];
        for i in 0..moves.len {
            scores.push({
                let m = moves.list[i];
                if m == hash_move {HASH_MOVE}
                else if m & 0x4000 > 0 {self.mvv_lva(m)}
                else if m & 0x8000 > 0 {PROMOTION + ((m & 0x7000) >> 12)}
                else if killers.contains(&m) {KILLER}
                else {0}
            })
        }
        scores
    }

    fn score_caps(&self, caps: &MoveList) -> MoveList {
        let mut scores = MoveList::uninit();
        for i in 0..caps.len {scores.push(self.mvv_lva(caps.list[i]))}
        scores
    }

    fn mvv_lva(&self, m: u16) -> u16 {
        let moved_pc: usize = self.squares[from!(m)] as usize;
        let captured_pc: usize = self.squares[to!(m)] as usize;
        MVV_LVA[captured_pc][moved_pc]
    }
}

impl MoveList {
    // O(n^2) algorithm to incrementally sort the move list as needed.
    fn pick(&mut self, scores: &mut MoveList) -> Option<(u16, u16)> {
        if scores.len == 0 {return None}
        let mut idx = 0;
        let mut best = 0;
        let mut score;
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

fn pvs(pos: &mut Pos, mut a: i16, mut b: i16, mut d: i8, ctx: &mut Ctx, nt: Nt, line: &mut Vec<u16>) -> i16 {
    // search aborting
    if ctx.abort { return 0 }
    if ctx.nodes & 1023 == 0 && ctx.timer() >= ctx.alloc_time {
        ctx.abort = true;
        return 0
    }

    if pos.is_draw(ctx.ply) { return 0 }
    let Nt(in_check, null) = nt;
    let pv = b > a + 1;

    // mate distance pruning
    a = max(a, -MAX + ctx.ply);
    b = min(b, MAX - ctx.ply - 1);
    if a >= b { return a }

    // check extensions
    d += i8::from(in_check);

    if d <= 0 || ctx.ply == MAX_PLY {
        ctx.seldepth = max(ctx.seldepth, ctx.ply);
        return qs(pos, a, b, ctx)
    }

    // probing hash table
    let hash = pos.state.hash;
    let mut bm = 0;
    let mut write = true;
    if let Some(res) = ctx.hash_table.probe(hash, ctx.ply) {
        write = d > res.depth;
        bm = res.best_move;
        if ctx.ply > 0 && res.depth >= d && pos.state.hfm < 90 && match res.bound {
            Bound::Lower => res.score >= b,
            Bound::Upper => res.score <= a,
            Bound::Exact => !pv, // want nice pv lines
        } { return res.score }
    }

    // pruning
    if !pv && !in_check && b.abs() < MATE {
        let eval = pos.eval();

        // reverse futility pruning
        let margin = eval - 120 * i16::from(d);
        if d <= 8 && margin >= b { return margin }

        // null move pruning
        if null && d >= 3 && pos.phase >= 6 && eval >= b {
            let enp = pos.do_null(&mut ctx.ply);
            let nw = -pvs(pos, -b, -b + 1, d - 3, ctx, Nt(false, false), &mut Vec::new());
            pos.undo_null(enp, hash, &mut ctx.ply);
            if nw >= b {return nw}
            if nw < -MATE {d += 1}
        }
    }

    ctx.nodes += 1;
    ctx.ply += 1;
    let lmr = d > 2 && ctx.ply > 1 && !in_check;
    let mut moves = pos.gen::<ALL>();
    let mut scores = pos.score(&moves, bm, ctx);
    let (mut legal, mut eval, mut bound) = (0, -MAX, Bound::Upper);
    let mut sline = Vec::new();
    while let Some((m, ms)) = moves.pick(&mut scores) {
        if pos.r#do(m) { continue }
        legal += 1;
        let check = pos.in_check();

        // late move reductions
        let r = i8::from(lmr && !check && legal > 2 && ms < KILLER);

        // principle variation search
        sline.clear();
        let score = if legal == 1 {
            -pvs(pos, -b, -a, d - 1, ctx, Nt(check, false), &mut sline)
        } else {
            let zw = -pvs(pos, -a - 1, -a, d - 1 - r, ctx, Nt(check, true), &mut sline);
            if (a != b - 1 || r > 0) && zw > a {
                -pvs(pos, -b, -a, d - 1, ctx, Nt(check, false), &mut sline)
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
                    if ms <= KILLER {ctx.killer_table.push(m, ctx.ply)}
                    break
                }
            }
        }
    }
    ctx.ply -= 1;
    if legal == 0 { return i16::from(in_check) * (-MAX + ctx.ply) }
    if write && !ctx.abort { ctx.hash_table.push(hash, bm, d, bound, eval, ctx.ply) }
    eval
}

fn qs(p: &mut Pos, mut a: i16, b: i16, ctx: &mut Ctx) -> i16 {
    ctx.qnodes += 1;
    let mut e: i16 = p.eval();
    if e >= b {return e}
    a = max(a, e);
    let mut caps = p.gen::<CAPTURES>();
    let mut scores = p.score_caps(&caps);
    while let Some((m, _)) = caps.pick(&mut scores) {
        if p.r#do(m) {continue}
        e = max(e, -qs(p, -b, -a, ctx));
        p.undo();
        if e >= b {break}
        a = max(a, e);
    }
    e
}

pub fn go(pos: &mut Pos, ctx: &mut Ctx) {
    ctx.reset();
    let mut best_move: u16 = 0;
    let in_check: bool = pos.in_check();
    for d in 1..=64 {
        let mut pv_line: Vec<u16> = Vec::with_capacity(d as usize);
        let score: i16 = pvs(pos, -MAX, MAX, d, ctx, Nt(in_check, false), &mut pv_line);
        if ctx.abort {break}
        best_move = pv_line[0];
        let (stype, sval): (&str, i16) = if score.abs() >= MATE {
            ("mate", if score < 0 {score.abs() - MAX} else {MAX - score + 1} / 2)
        } else {("cp", score)};
        let t: u128 = ctx.timer();
        let nodes: u64 = ctx.nodes + ctx.qnodes;
        let nps: u32 = ((nodes as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = pv_line.iter().map(|&m: &u16| u16_to_uci(m)).collect::<String>();
        println!("info depth {d} seldepth {} score {stype} {sval} time {t} nodes {nodes} nps {nps} pv {pv_str}", ctx.seldepth);
    }
    println!("bestmove {}", u16_to_uci(best_move));
}
