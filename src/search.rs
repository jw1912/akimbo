use std::{cmp::{max, min}, time::Instant};
use super::{consts::*, position::*, movegen::*, tables::*};

pub struct Timer(Instant, pub u128);
impl Default for Timer {
    fn default() -> Self {
        Timer(Instant::now(), 1000)
    }
}

#[derive(Default)]
pub struct Engine {
    pub timing: Timer,
    pub ttable: HashTable,
    pub htable: Box<HistoryTable>,
    ktable: Box<KillerTable>,
    pub zvals: Box<ZobristVals>,
    stack: Vec<u64>,
    nodes: u64,
    qnodes: u64,
    ply: i16,
    abort: bool,
}

impl Engine {
    fn reset(&mut self) {
        self.timing.0 = Instant::now();
        self.nodes = 0;
        self.qnodes = 0;
        self.ply = 0;
        self.abort = false;
    }

    fn score(&self, pos: &Position, moves: &MoveList, hash_move: Move) -> ScoreList {
        let mut scores = ScoreList::uninit();
        let killers = self.ktable.0[self.ply as usize];
        for &m in &moves.list[0..moves.len] {
            scores.add(
                if m == hash_move { HASH }
                else if m.flag == ENP { 2 * MVV_LVA }
                else if m.flag & 4 > 0 { self.mvv_lva(m, pos) }
                else if m.flag & 8 > 0 { PROMOTION + i16::from(m.flag & 7) }
                else if killers.contains(&m) { KILLER }
                else {self.htable.score(pos.c, m)}
            );
        }
        scores
    }

    fn score_caps(&self, caps: &MoveList, pos: &Position) -> ScoreList {
        let mut scores = ScoreList::uninit();
        for i in 0..caps.len { scores.add(self.mvv_lva(caps.list[i], pos)) }
        scores
    }

    #[inline]
    fn mvv_lva(&self, m: Move, pos: &Position) -> i16 {
        MVV_LVA * pos.get_pc(1 << m.to) as i16 - m.mpc as i16
    }

    fn rep_draw(&self, pos: &Position) -> bool {
        let mut num = 1 + 2 * u8::from(self.ply == 0);
        let l = self.stack.len();
        if l < 6 || pos.nulls > 0 { return false }
        for &hash in self.stack.iter().rev().take(pos.hfm as usize + 1).skip(1).step_by(2) {
            num -= u8::from(hash == pos.hash);
            if num == 0 { return true }
        }
        false
    }
}

pub fn go(pos: &Position, eng: &mut Engine) {
    eng.reset();
    let mut best_move = Move::default();
    let in_check: bool = pos.in_check();

    for d in 1..=64 {
        let mut pv_line = Vec::with_capacity(d as usize);
        let score = search(pos, eng, -MAX, MAX, d, in_check, false, &mut pv_line);
        if eng.abort { break }
        best_move = pv_line[0];

        // UCI output
        let (stype, sval) = if score.abs() >= MATE {
            ("mate", if score < 0 {score.abs() - MAX} else {MAX - score + 1} / 2)
        } else {("cp", score)};
        let t = eng.timing.0.elapsed();
        let nodes = eng.nodes + eng.qnodes;
        let nps = ((nodes as f64) / t.as_secs_f64()) as u32;
        let pv_str = pv_line.iter().map(|&m| m.to_uci()).collect::<String>();
        println!("info depth {d} score {stype} {sval} time {} nodes {nodes} nps {nps} pv {pv_str}", t.as_millis());
    }

    println!("bestmove {}", best_move.to_uci());
    *eng.ktable = Default::default();
    eng.htable.age();
}

fn qsearch(pos: &Position, eng: &mut Engine, mut alpha: i16, beta: i16) -> i16 {
    eng.qnodes += 1;
    let mut eval = pos.lazy_eval();
    if eval >= beta { return eval }
    alpha = max(alpha, eval);
    let mut caps = pos.gen::<CAPTURES>();
    let mut scores = eng.score_caps(&caps, pos);
    while let Some((r#move, _)) = caps.pick(&mut scores) {
        let mut new_pos = *pos;
        if new_pos.make(r#move, &eng.zvals) { continue }
        eval = max(eval, -qsearch(&new_pos, eng, -beta, -alpha));
        if eval >= beta { break }
        alpha = max(alpha, eval);
    }
    eval
}

fn search(pos: &Position, eng: &mut Engine, mut alpha: i16, mut beta: i16, mut depth: i8, in_check: bool, null: bool, line: &mut Vec<Move>) -> i16 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.0.elapsed().as_millis() >= eng.timing.1 {
        eng.abort = true;
        return 0
    }

    line.clear();

    // draw detection
    if pos.hfm >= 100 || pos.mat_draw() || eng.rep_draw(pos) { return 0 }

    // mate distance pruning
    alpha = max(alpha, -MAX + eng.ply);
    beta = min(beta, MAX - eng.ply - 1);
    if alpha >= beta { return alpha }

    // check extensions - not on root
    depth += i8::from(in_check && eng.ply > 0);

    // drop into quiescence search?
    if depth <= 0 || eng.ply == MAX_PLY { return qsearch(pos, eng, alpha, beta) }

    eng.nodes += 1;
    let (pv_node, hash) = (beta > alpha + 1, pos.hash(&eng.zvals));
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
    if !pv_node && !in_check && beta.abs() < MATE {
        let eval = pos.lazy_eval();

        // reverse futility pruning?
        let margin = eval - 120 * i16::from(depth);
        if depth <= 8 && margin >= beta { return margin }

        // null move pruning?
        if null && depth >= 3 && pos.phase >= 6 && eval >= beta {
            let r = 2 + depth / 3;
            eng.ply += 1;
            eng.stack.push(pos.hash);
            let mut new_pos = *pos;
            new_pos.r#do_null();
            let nw = -search(&new_pos, eng, -alpha - 1, -alpha, depth - r, false, false, &mut Vec::new());
            eng.stack.pop();
            eng.ply -= 1;
            if nw >= MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    let mut moves = pos.gen::<ALL>();
    let mut scores = eng.score(pos, &moves, best_move);
    let can_lmr = depth > 1 && eng.ply > 0 && !in_check;
    let (mut legal, mut eval, mut bound, mut sline) = (0, -MAX, UPPER, Vec::new());

    eng.ply += 1;
    eng.stack.push(pos.hash);
    while let Some((r#move, mscore)) = moves.pick(&mut scores) {
        let mut new_pos = *pos;
        if new_pos.make(r#move, &eng.zvals) { continue }
        let check = new_pos.in_check();
        legal += 1;

        // late move reductions - Viridithas values used
        let reduce = if can_lmr && !check && mscore < KILLER {
            let lmr = (0.77 + f64::from(depth).ln() * f64::from(legal).ln() / 2.67) as i8;
            if pv_node { max(1, lmr - 1) } else { lmr }
        } else {0};

        // pvs framework
        let score = if legal == 1 {
            -search(&new_pos, eng, -beta, -alpha, depth - 1, check, false, &mut sline)
        } else {
            let zw = -search(&new_pos, eng, -alpha - 1, -alpha, depth - 1 - reduce, check, true, &mut sline);
            if zw > alpha && (pv_node || reduce > 0) {
                -search(&new_pos, eng, -beta, -alpha, depth - 1, check, false, &mut sline)
            } else { zw }
        };

        // best move so far?
        if score <= eval { continue }
        eval = score;
        best_move = r#move;

        // update pv line
        line.clear();
        line.push(r#move);
        line.append(&mut sline);

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
    eng.stack.pop();
    eng.ply -= 1;

    if legal == 0 { return i16::from(in_check) * (-MAX + eng.ply) }
    if write && !eng.abort { eng.ttable.push(hash, best_move, depth, bound, eval, eng.ply) }
    eval
}