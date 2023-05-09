use std::{sync::atomic::{AtomicU64, Ordering::Relaxed}, time::Instant};
use super::{consts::*, position::*, movegen::*, tables::*};

static QNODES: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub struct Engine {
    timing: Option<Instant>,
    pub max_time: u128,
    pub ttable: HashTable,
    pub htable: Box<HistoryTable>,
    ktable: Box<KillerTable>,
    pub stack: Vec<u64>,
    nodes: u64,
    ply: i16,
    abort: bool,
    best_move: Move,
}

#[inline]
fn mvv_lva(m: Move, pos: &Position) -> i16 {
    MVV_LVA * pos.get_pc(1 << m.to) as i16 - m.mpc as i16
}

impl Engine {
    fn rep_draw(&self, pos: &Position, curr_hash: u64) -> bool {
        let mut num = 1 + u8::from(self.ply == 0);
        if self.stack.len() < 6 || pos.nulls > 0 { return false }
        for &hash in self.stack.iter().rev().take(pos.hfm as usize + 1).skip(1).step_by(2) {
            num -= u8::from(hash == curr_hash);
            if num == 0 { return true }
        }
        false
    }

    fn push(&mut self, hash: u64) {
        self.ply += 1;
        self.stack.push(hash);
    }

    fn pop(&mut self) {
        self.stack.pop();
        self.ply -= 1;
    }
}

pub fn go(start: &Position, eng: &mut Engine) {
    // reset engine
    eng.timing = Some(Instant::now());
    eng.nodes = 0;
    eng.ply = 0;
    eng.best_move = Move::default();
    eng.abort = false;
    QNODES.store(0, Relaxed);

    let mut best = String::new();
    let mut pos = *start;
    pos.check = pos.in_check();

    // iterative deepening loop
    for d in 1..=64 {
        let eval = pvs(&pos, eng, -MAX, MAX, d, false);
        if eng.abort { break }
        best = eng.best_move.to_uci();

        // UCI output
        let score = if eval.abs() >= MATE {
            format!("score mate {: <2}", if eval < 0 {eval.abs() - MAX} else {MAX - eval + 1} / 2)
        } else {format!("score cp {: <4}", eval)};
        let t = eng.timing.unwrap().elapsed();
        let nodes = eng.nodes + QNODES.load(Relaxed);
        let nps = ((nodes as f64) / t.as_secs_f64()) as u32;
        println!("info depth {d: <2} {score} time {: <5} nodes {nodes: <9} nps {nps: <8.0} pv {best}", t.as_millis());
    }

    println!("bestmove {best}");
    *eng.ktable = Default::default();
    eng.htable.age();
}

fn qs(pos: &Position, mut alpha: i16, beta: i16) -> i16 {
    let mut eval = pos.lazy_eval();
    if eval >= beta { return eval }
    alpha = alpha.max(eval);

    let mut caps = pos.gen::<CAPTURES>();
    let mut scores = ScoreList::default();
    for i in 0..caps.len { scores.list[i] = mvv_lva(caps.list[i], pos) }

    while let Some((mov, _)) = caps.pick(&mut scores) {
        let mut new = *pos;
        if new.make(mov) { continue }
        QNODES.fetch_add(1, Relaxed);

        eval = eval.max(-qs(&new, -beta, -alpha));

        if eval >= beta { break }
        alpha = alpha.max(eval);
    }

    eval
}

fn pvs(pos: &Position, eng: &mut Engine, alpha: i16, beta: i16, depth: i8, null: bool) -> i16 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.unwrap().elapsed().as_millis() >= eng.max_time {
        eng.abort = true;
        return 0
    }

    // draw detection
    let hash = pos.hash();
    if pos.hfm >= 100 || pos.mat_draw() || eng.rep_draw(pos, hash) { return 0 }

    // mate distance pruning
    let mut alpha = alpha.max(-MAX + eng.ply);
    let beta = beta.min(MAX - eng.ply - 1);
    if alpha >= beta { return alpha }

    // check extensions - not on root
    let depth = depth + i8::from(pos.check && eng.ply > 0);

    // drop into quiescence search
    if depth <= 0 || eng.ply == MAX_PLY { return qs(pos, alpha, beta) }

    // probing hash table
    let pv_node = beta > alpha + 1;
    let (mut best_move, mut write) = (Move::default(), true);
    if let Some(res) = eng.ttable.probe(hash, eng.ply) {
        write = depth > res.depth;
        best_move = Move::from_short(res.best_move, pos);

        // hash score pruning
        if !pv_node && !write && match res.bound {
            LOWER => res.score >= beta,
            UPPER => res.score <= alpha,
            _ => true,
        } { return res.score }
    }

    // pruning
    if !pv_node && !pos.check && beta.abs() < MATE {
        let eval = pos.lazy_eval();

        // reverse futility pruning
        if depth <= 8 && eval >= beta + 120 * i16::from(depth) { return eval }

        // razoring
        if depth <= 2 && eval + 400 * i16::from(depth) < alpha {
            let qeval = qs(pos, alpha, beta);
            if qeval < alpha { return qeval }
        }

        // null move pruning
        if null && depth >= 3 && pos.phase >= 6 && eval >= beta {
            let mut new = *pos;
            let r = 3 + depth / 3;
            eng.push(hash);
            new.nulls += 1;
            new.c = !new.c;
            new.enp = 0;
            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false);
            eng.pop();
            if nw >= MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    // generating and scoring moves
    let mut moves = pos.gen::<ALL>();
    let mut scores = ScoreList::default();
    let killers = eng.ktable.0[eng.ply as usize];
    for (i, &m) in moves.list[0..moves.len].iter().enumerate() {
        scores.list[i] = if m == best_move { HASH }
            else if m.flag == ENP { 2 * MVV_LVA }
            else if m.flag & 4 > 0 { mvv_lva(m, pos) }
            else if m.flag & 8 > 0 { PROMOTION + i16::from(m.flag & 7) }
            else if killers.contains(&m) { KILLER }
            else { eng.htable.score(pos.c, m) };
    }

    // stuff for going through moves
    let (mut legal, mut eval, mut bound) = (0, -MAX, UPPER);
    let can_lmr = depth > 1 && eng.ply > 0 && !pos.check;

    eng.push(hash);
    while let Some((mov, ms)) = moves.pick(&mut scores) {
        let mut new = *pos;
        if new.make(mov) { continue }
        new.check = new.in_check();
        eng.nodes += 1;
        legal += 1;

        // late move reductions - Viridithas values used
        let reduce = if can_lmr && !new.check && ms < KILLER {
            let lmr = (0.77 + (depth as f64).ln() * (legal as f64).ln() / 2.67) as i8;
            if pv_node { 0.max(lmr - 1) } else { lmr }
        } else { 0 };

        let score = if legal == 1 {
            -pvs(&new, eng, -beta, -alpha, depth - 1, false)
        } else {
            let zw = -pvs(&new, eng, -alpha - 1, -alpha, depth - 1 - reduce, true);
            if zw > alpha && (pv_node || reduce > 0) {
                -pvs(&new, eng, -beta, -alpha, depth - 1, false)
            } else { zw }
        };

        if score <= eval { continue }
        eval = score;
        best_move = mov;

        if score <= alpha { continue }
        alpha = score;
        bound = EXACT;

        if score < beta { continue }
        bound = LOWER;

        // quiet cutoffs pushed to tables
        if mov.flag >= CAP || eng.abort { break }
        eng.ktable.push(mov, eng.ply);
        eng.htable.push(mov, pos.c, depth);

        break
    }
    eng.pop();

    // end of node shenanigans
    if eng.abort { return 0 }
    if eng.ply == 0 { eng.best_move = best_move }
    if legal == 0 { return i16::from(pos.check) * (-MAX + eng.ply) }
    if write { eng.ttable.push(hash, best_move, depth, bound, eval, eng.ply) }
    eval
}