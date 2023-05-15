use std::{sync::atomic::{AtomicU64, Ordering::Relaxed}, time::Instant};
use super::{util::*, position::*, movegen::*, tables::*};

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
fn mvv_lva(mov: Move, pos: &Position) -> i16 {
    Score::MVV_LVA * pos.get_pc(1 << mov.to) as i16 - mov.pc as i16
}

impl Engine {
    fn rep_draw(&self, pos: &Position, curr_hash: u64) -> bool {
        if self.stack.len() < 6 || pos.nulls > 0 { return false }
        for &hash in self.stack.iter().rev().take(pos.halfm as usize + 1).skip(1).step_by(2) {
            if hash == curr_hash { return true }
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
    *eng.ktable = Default::default();
    eng.htable.age();
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
        let mut pv_line = Vec::new();
        let eval = pvs(&pos, eng, -Score::MAX, Score::MAX, d, false, &mut pv_line);
        if eng.abort { break }
        best = eng.best_move.to_uci();

        // UCI output
        let score = if eval.abs() >= Score::MATE {
            format!("score mate {}", if eval < 0 {eval.abs() - Score::MAX} else {Score::MAX - eval + 1} / 2)
        } else {format!("score cp {eval}")};
        let t = eng.timing.unwrap().elapsed().as_millis();
        let nodes = eng.nodes + QNODES.load(Relaxed);
        let nps = (1000.0 * nodes as f64 / t as f64) as u32;
        let pv = pv_line.iter().map(|mov| mov.to_uci()).collect::<String>();
        println!("info depth {d} {score} time {t} nodes {nodes} nps {nps:.0} pv {pv}");
    }
    eng.ttable.age();
    println!("bestmove {best}");
}

fn qs(pos: &Position, mut alpha: i16, beta: i16) -> i16 {
    let mut eval = pos.eval();
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

fn pvs(pos: &Position, eng: &mut Engine, mut alpha: i16, mut beta: i16, mut depth: i8, null: bool, line: &mut Vec<Move>) -> i16 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.unwrap().elapsed().as_millis() >= eng.max_time {
        eng.abort = true;
        return 0
    }

    line.clear();
    let hash = pos.hash();

    if eng.ply > 0 {
        // draw detection
        if pos.halfm >= 100 || pos.mat_draw() || eng.rep_draw(pos, hash) { return 0 }

        // mate distance pruning
        alpha = alpha.max(eng.ply - Score::MAX);
        beta = beta.min(Score::MAX - eng.ply - 1);
        if alpha >= beta { return alpha }

        // check extensions - not on root
        depth += i8::from(pos.check);
    }

    // drop into quiescence search
    if depth <= 0 || eng.ply == MAX_PLY { return qs(pos, alpha, beta) }

    // probing hash table
    let pv_node = beta > alpha + 1;
    let mut best_move = Move::default();
    if let Some(res) = eng.ttable.probe(hash, eng.ply) {
        best_move = Move::from_short(res.best_move, pos);
        if !pv_node && depth <= res.depth && match res.bound & 3 {
            Bound::LOWER => res.score >= beta,
            Bound::UPPER => res.score <= alpha,
            _ => true,
        } { return res.score }
    }

    // pruning
    if !pv_node && !pos.check && beta.abs() < Score::MATE {
        let eval = pos.eval();

        // reverse futility pruning
        if depth <= 8 && eval >= beta + 120 * i16::from(depth) { return eval }

        // razoring
        if depth <= 2 && eval + 400 * i16::from(depth) < alpha {
            let qeval = qs(pos, alpha, beta);
            if qeval < alpha { return qeval }
        }

        // null move pruning
        if null && depth >= 3 && pos.phase > 2 && eval >= beta {
            let mut new = *pos;
            let r = 3 + depth / 3;
            eng.push(hash);
            new.nulls += 1;
            new.c = !new.c;
            new.enp_sq = 0;
            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false, &mut Vec::new());
            eng.pop();
            if nw >= Score::MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    // internal iterative reduction
    if depth >= 4 && best_move == Move::default() { depth -= 1 }

    // generating and scoring moves
    let mut moves = pos.gen::<ALL>();
    let mut scores = ScoreList::default();
    let killers = eng.ktable.0[eng.ply as usize];
    for (i, &mov) in moves.list[..moves.len].iter().enumerate() {
        scores.list[i] = if mov == best_move { Score::HASH }
            else if mov.flag == Flag::ENP { 2 * Score::MVV_LVA }
            else if mov.flag & 4 > 0 { mvv_lva(mov, pos) }
            else if mov.flag & 8 > 0 { Score::PROMO + i16::from(mov.flag & 7) }
            else if killers.contains(&mov) { Score::KILLER }
            else { eng.htable.score(mov, pos.c) };
    }

    // stuff for going through moves
    let (mut legal, mut eval, mut bound, mut sline) = (0, -Score::MAX, Bound::UPPER, Vec::new());
    let can_lmr = depth > 1 && eng.ply > 0 && !pos.check;
    let lmr_base = (depth as f64).ln() / 2.67;

    eng.push(hash);
    while let Some((mov, ms)) = moves.pick(&mut scores) {
        let mut new = *pos;
        if new.make(mov) { continue }
        new.check = new.in_check();
        eng.nodes += 1;
        legal += 1;

        // late move reductions - Viridithas values used
        let reduce = if can_lmr && !new.check && ms < Score::KILLER {
            let lmr = (0.77 + lmr_base * (legal as f64).ln()) as i8;
            if pv_node { 0.max(lmr - 1) } else { lmr }
        } else { 0 };

        let score = if legal == 1 {
            -pvs(&new, eng, -beta, -alpha, depth - 1, false, &mut sline)
        } else {
            let zw = -pvs(&new, eng, -alpha - 1, -alpha, depth - 1 - reduce, true, &mut sline);
            if zw > alpha && (pv_node || reduce > 0) {
                -pvs(&new, eng, -beta, -alpha, depth - 1, false, &mut sline)
            } else { zw }
        };

        if score <= eval { continue }
        eval = score;
        best_move = mov;
        if pv_node {
            line.clear();
            line.push(mov);
            line.append(&mut sline);
        }

        if score <= alpha { continue }
        alpha = score;
        bound = Bound::EXACT;

        if score < beta { continue }
        bound = Bound::LOWER;

        // quiet cutoffs pushed to tables
        if mov.flag >= Flag::CAP || eng.abort { break }
        eng.ktable.push(mov, eng.ply);
        eng.htable.push(mov, pos.c, depth);

        break
    }
    eng.pop();

    // end of node shenanigans
    if eng.abort { return 0 }
    if eng.ply == 0 { eng.best_move = best_move }
    if legal == 0 { return i16::from(pos.check) * (-Score::MAX + eng.ply) }
    eng.ttable.push(hash, best_move, depth, bound, eval, eng.ply);
    eval
}