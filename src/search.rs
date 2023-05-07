use std::time::Instant;
use super::{consts::*, position::*, movegen::*, tables::*};

#[derive(Default)]
pub struct Engine {
    timing: Option<Instant>,
    pub max_time: u128,
    pub ttable: HashTable,
    pub htable: Box<HistoryTable>,
    ktable: Box<KillerTable>,
    pub stack: Vec<u64>,
    nodes: u64,
    qnodes: u64,
    ply: i16,
    abort: bool,
    best_move: Move,
    nulls: i16,
}

impl Engine {
    fn reset(&mut self) {
        self.timing = Some(Instant::now());
        self.nodes = 0;
        self.qnodes = 0;
        self.ply = 0;
        self.best_move = Move::default();
        self.abort = false;
    }

    fn score(&self, pos: &Position, moves: &MoveList, hash_move: Move) -> ScoreList {
        let mut scores = ScoreList::uninit();
        let killers = self.ktable.0[self.ply as usize];
        for (i, &m) in moves.list[0..moves.len].iter().enumerate() {
            scores.list[i] =
                if m == hash_move { HASH }
                else if m.flag == ENP { 2 * MVV_LVA }
                else if m.flag & 4 > 0 { self.mvv_lva(m, pos) }
                else if m.flag & 8 > 0 { PROMOTION + i16::from(m.flag & 7) }
                else if killers.contains(&m) { KILLER }
                else { self.htable.score(pos.c, m) };
        }
        scores
    }

    fn score_caps(&self, caps: &MoveList, pos: &Position) -> ScoreList {
        let mut scores = ScoreList::uninit();
        for i in 0..caps.len { scores.list[i] = self.mvv_lva(caps.list[i], pos) }
        scores
    }

    #[inline]
    fn mvv_lva(&self, m: Move, pos: &Position) -> i16 {
        MVV_LVA * pos.get_pc(1 << m.to) as i16 - m.mpc as i16
    }

    fn rep_draw(&self, pos: &Position, curr_hash: u64) -> bool {
        let mut num = 1 + u8::from(self.ply == 0);
        let l = self.stack.len();
        if l < 6 || self.nulls > 0 { return false }
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
    eng.reset();
    let mut best = String::new();
    let mut pos = *start;
    pos.check = pos.in_check();

    for d in 1..=64 {
        let eval = search(&pos, eng, -MAX, MAX, d, false);
        if eng.abort { break }
        best = eng.best_move.to_uci();

        // UCI output
        let score = if eval.abs() >= MATE {
            format!("score mate {: <2}", if eval < 0 {eval.abs() - MAX} else {MAX - eval + 1} / 2)
        } else {format!("score cp {: <4}", eval)};
        let t = eng.timing.unwrap().elapsed();
        let nodes = eng.nodes + eng.qnodes;
        let nps = ((nodes as f64) / t.as_secs_f64()) as u32;
        println!("info depth {d: <2} {score} time {: <5} nodes {nodes: <9} nps {nps: <8.0} pv {best}", t.as_millis());
    }

    println!("bestmove {best}");
    *eng.ktable = Default::default();
    eng.htable.age();
}

fn qsearch(pos: &Position, eng: &mut Engine, mut alpha: i16, beta: i16) -> i16 {
    let mut eval = pos.lazy_eval();
    if eval >= beta { return eval }
    alpha = alpha.max(eval);
    let mut caps = pos.gen::<CAPTURES>();
    let mut scores = eng.score_caps(&caps, pos);
    while let Some((r#move, _)) = caps.pick(&mut scores) {
        let mut new_pos = *pos;
        if new_pos.make(r#move) { continue }
        eng.qnodes += 1;
        eval = eval.max(-qsearch(&new_pos, eng, -beta, -alpha));
        if eval >= beta { break }
        alpha = alpha.max(eval);
    }
    eval
}

fn search(pos: &Position, eng: &mut Engine, mut alpha: i16, mut beta: i16, mut depth: i8, null: bool) -> i16 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.unwrap().elapsed().as_millis() >= eng.max_time {
        eng.abort = true;
        return 0
    }

    // calculate full hash
    let hash = pos.hash();

    // draw detection
    if pos.hfm >= 100 || pos.mat_draw() || eng.rep_draw(pos, hash) { return 0 }

    // mate distance pruning
    alpha = alpha.max(-MAX + eng.ply);
    beta = beta.min(MAX - eng.ply - 1);
    if alpha >= beta { return alpha }

    // check extensions - not on root
    depth += i8::from(pos.check && eng.ply > 0);

    // drop into quiescence search?
    if depth <= 0 || eng.ply == MAX_PLY { return qsearch(pos, eng, alpha, beta) }

    let pv_node = beta > alpha + 1;
    let (mut best_move, mut write) = (Move::default(), true);

    // probing hash table
    if let Some(res) = eng.ttable.probe(hash, eng.ply) {
        write = depth > res.depth;
        best_move = Move::from_short(res.best_move, pos);

        // hash score pruning?
        if !pv_node && res.depth >= depth && match res.bound {
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
            let qeval = qsearch(pos, eng, alpha, beta);
            if qeval < alpha { return qeval }
        }

        // null move pruning
        if null && depth >= 3 && pos.phase >= 6 && eval >= beta {
            let mut new_pos = *pos;
            let r = 3 + depth / 3;
            eng.push(hash);
            eng.nulls += 1;
            new_pos.c = !new_pos.c;
            new_pos.enp = 0;
            let nw = -search(&new_pos, eng, -alpha - 1, -alpha, depth - r, false);
            eng.nulls -= 1;
            eng.pop();
            if nw >= MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    // stuff for going through moves
    let mut moves = pos.gen::<ALL>();
    let mut scores = eng.score(pos, &moves, best_move);
    let (mut legal, mut eval, mut bound) = (0, -MAX, UPPER);

    // pruning/reductions allowed?
    let can_lmr = depth > 1 && eng.ply > 0 && !pos.check;

    eng.push(hash);
    while let Some((r#move, mscore)) = moves.pick(&mut scores) {
        // copy position, make move and skip if not legal
        let mut new_pos = *pos;
        if new_pos.make(r#move) { continue }

        // update stuff
        new_pos.check = new_pos.in_check();
        eng.nodes += 1;
        legal += 1;

        // late move reductions - Viridithas values used
        let reduce = if can_lmr && !new_pos.check && mscore < KILLER {
            let lmr = (0.77 + (depth as f64).ln() * (legal.min(63) as f64).ln() / 2.67) as i8;
            if pv_node { 0.max(lmr - 1) } else { lmr }
        } else {0};

        // pvs framework
        let score = if legal == 1 {
            -search(&new_pos, eng, -beta, -alpha, depth - 1, false)
        } else {
            let zw = -search(&new_pos, eng, -alpha - 1, -alpha, depth - 1 - reduce, true);
            if zw > alpha && (pv_node || reduce > 0) {
                -search(&new_pos, eng, -beta, -alpha, depth - 1, false)
            } else { zw }
        };

        // best move so far?
        if score <= eval { continue }
        eval = score;
        best_move = r#move;

        // improve alpha?
        if score <= alpha { continue }
        alpha = score;
        bound = EXACT;

        // beta cutoff?
        if score < beta { continue }
        bound = LOWER;

        // quiet cutoffs pushed to tables
        if r#move.flag >= CAP { break }
        eng.ktable.push(r#move, eng.ply);
        eng.htable.push(r#move, pos.c, depth);

        break
    }
    eng.pop();

    // don't trust results if search was aborted during the node
    if eng.abort { return 0 }

    // record best move at root
    if eng.ply == 0 { eng.best_move = best_move }

    // (stale/check)mate
    if legal == 0 { return i16::from(pos.check) * (-MAX + eng.ply) }

    // writing to hash table
    if write { eng.ttable.push(hash, best_move, depth, bound, eval, eng.ply) }

    eval
}