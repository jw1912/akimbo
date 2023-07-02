use std::{sync::atomic::{AtomicU64, Ordering::Relaxed}, time::Instant};
use super::{util::{Bound, Flag, MoveScore, Piece, Score, SPANS}, position::{Move, MoveList, Position}};

pub static QNODES: AtomicU64 = AtomicU64::new(0);

fn mvv_lva(mov: Move, pos: &Position) -> i32 {
    MoveScore::HISTORY_MAX * pos.get_pc(1 << mov.to) as i32 - mov.pc as i32
}

#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    key: u16,
    best_move: u16,
    score: i16,
    depth: i8,
    bound: u8,
}

pub struct Engine {
    // search control
    pub timing: Instant,
    pub max_time: u128,
    pub abort: bool,

    // tables
    pub tt: Vec<HashEntry>,
    pub tt_age: u8,
    pub htable: Box<[[[i32; 64]; 6]; 2]>,
    pub ktable: Box<[[Move; 2]; 96]>,
    pub evals: Box<[i32; 96]>,
    pub stack: Vec<u64>,

    // uci output
    pub nodes: u64,
    pub ply: i32,
    pub best_move: Move,
    pub pv_table: Box<[MoveList; 96]>,
}

impl Engine {
    fn repetition(&self, pos: &Position, curr_hash: u64, root: bool) -> bool {
        if self.stack.len() < 6 { return false }
        let mut reps = 1 + u8::from(root);
        for &hash in self.stack.iter().rev().take(pos.halfm as usize + 1).skip(1).step_by(2) {
            reps -= u8::from(hash == curr_hash);
            if reps == 0 { return true }
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

    pub fn resize_tt(&mut self, size: usize) {
        self.tt = vec![HashEntry::default(); 1 << (80 - (size as u64).leading_zeros())];
        self.tt_age = 0;
    }

    pub fn clear_tt(&mut self) {
        self.tt.iter_mut().for_each(|entry| *entry = HashEntry::default());
        self.tt_age = 0;
    }

    fn push_tt(&mut self, hash: u64, mov: Move, depth: i32, bound: u8, mut score: i32) {
        let (key, idx) = ((hash >> 48) as u16, (hash as usize) & (self.tt.len() - 1));
        let entry = &mut self.tt[idx];

        // replacement scheme
        let diff = self.tt_age - (entry.bound >> 2);
        if self.ply > 0 && key == entry.key && depth as u8 + 2 * diff < entry.depth as u8  { return }

        // replace entry
        score += if score.abs() > Score::MATE {score.signum() * self.ply} else {0};
        let best_move = u16::from(mov.from) << 6 | u16::from(mov.to) | u16::from(mov.flag) << 12;
        *entry = HashEntry { key, best_move, score: score as i16, depth: depth as i8, bound: (self.tt_age << 2) | bound };
    }

    fn probe_tt(&self, hash: u64) -> Option<HashEntry> {
        let mut entry = self.tt[(hash as usize) & (self.tt.len() - 1)];
        if entry.key != (hash >> 48) as u16 { return None }
        entry.score -= if entry.score.abs() > Score::MATE as i16 {entry.score.signum() * self.ply as i16} else {0};
        Some(entry)
    }

    fn push_killer(&mut self, m: Move) {
        let ply = self.ply as usize - 1;
        self.ktable[ply][1] = self.ktable[ply][0];
        self.ktable[ply][0] = m;
    }

    fn push_history(&mut self, mov: Move, side: bool, bonus: i32) {
        let entry = &mut self.htable[usize::from(side)][usize::from(mov.pc - 2)][usize::from(mov.to)];
        *entry += bonus - *entry * bonus.abs() / MoveScore::HISTORY_MAX
    }
}

pub fn go(start: &Position, eng: &mut Engine, report: bool, max_depth: i32) {
    // reset engine
    *eng.ktable = [[Move::default(); 2]; 96];
    eng.htable.iter_mut().flatten().flatten().for_each(|x| *x /= 2);
    eng.timing = Instant::now();
    eng.nodes = 0;
    eng.ply = 0;
    eng.best_move = Move::default();
    eng.abort = false;
    QNODES.store(0, Relaxed);

    let mut best_move = Move::default();
    let mut pos = *start;
    let mut eval = 0;
    pos.check = pos.in_check();

    // iterative deepening loop
    for d in 1..=max_depth {
        eval = if d < 7 {
            pvs(&pos, eng, -Score::MAX, Score::MAX, d, false)
        } else { aspiration(&pos, eng, eval, d, &mut best_move) };

        if eng.abort { break }
        best_move = eng.best_move;

        // UCI output
        if report {
            let score = if eval.abs() >= Score::MATE {
                format!("score mate {}", if eval < 0 {eval.abs() - Score::MAX} else {Score::MAX - eval + 1} / 2)
            } else {format!("score cp {eval}")};
            let t = eng.timing.elapsed().as_millis();
            let nodes = eng.nodes + QNODES.load(Relaxed);
            let nps = (1000.0 * nodes as f64 / t as f64) as u32;
            let pv_line = &eng.pv_table[0];
            let pv = pv_line.list.iter().take(pv_line.len).map(|mov| format!("{} ", mov.to_uci())).collect::<String>();
            println!("info depth {d} {score} time {t} nodes {nodes} nps {nps:.0} pv {pv}");
        }
    }
    eng.tt_age = 63.min(eng.tt_age + 1);
    println!("bestmove {}", best_move.to_uci());
}

fn aspiration(pos: &Position, eng: &mut Engine, mut score: i32, depth: i32, best_move: &mut Move) -> i32 {
    let mut delta = 25;
    let mut alpha = (-Score::MAX).max(score - delta);
    let mut beta = Score::MAX.min(score + delta);

    loop {
        score = pvs(pos, eng, alpha, beta, depth, false);
        if eng.abort { return 0 }

        if score <= alpha {
            beta = (alpha + beta) / 2;
            alpha = (-Score::MAX).max(alpha - delta);
        } else if score >= beta {
            alpha = (alpha + beta) / 2;
            beta = Score::MAX.min(beta + delta);
            *best_move = eng.best_move;
        } else {
            return score
        }

        delta *= 2;
    }
}

fn qs(pos: &Position, mut alpha: i32, beta: i32) -> i32 {
    let mut eval = pos.eval();
    if eval >= beta { return eval }
    alpha = alpha.max(eval);

    let mut caps = pos.movegen::<false>();
    let mut scores = [0; 252];
    for (i, score) in scores.iter_mut().enumerate().take(caps.len) {
        *score = mvv_lva(caps.list[i], pos)
    }

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

fn pvs(pos: &Position, eng: &mut Engine, mut alpha: i32, mut beta: i32, mut depth: i32, null: bool) -> i32 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.elapsed().as_millis() >= eng.max_time {
        eng.abort = true;
        return 0
    }

    eng.pv_table[eng.ply as usize].len = 0;
    let hash = pos.hash();

    if eng.ply > 0 {
        // draw detection
        if pos.draw() || eng.repetition(pos, hash, eng.ply == 0) { return Score::DRAW }

        // mate distance pruning
        alpha = alpha.max(eng.ply - Score::MAX);
        beta = beta.min(Score::MAX - eng.ply - 1);
        if alpha >= beta { return alpha }

        // check extensions - not on root
        depth += i32::from(pos.check);
    }

    // drop into quiescence search
    if depth <= 0 || eng.ply == 96 { return qs(pos, alpha, beta) }

    // probing hash table
    let pv_node = beta > alpha + 1;
    let mut eval = pos.eval();
    let mut tt_hit = false;
    let mut tt_move = Move::default();
    if let Some(res) = eng.probe_tt(hash) {
        tt_hit = true;
        tt_move = Move::from_short(res.best_move, pos);
        let tt_score = i32::from(res.score);
        if !pv_node && depth <= i32::from(res.depth)
            && match res.bound & 3 {
                Bound::LOWER => tt_score >= beta,
                Bound::UPPER => tt_score <= alpha,
                _ => true,
        } { return tt_score }
        if (eval <= tt_score || res.bound & 3 != Bound::LOWER)
            && (eval >= tt_score || res.bound & 3 != Bound::UPPER) {
                eval = tt_score;
        }
    }

    // improving heuristic
    eng.evals[eng.ply as usize] = eval;
    let improving = eng.ply > 1 && eval > eng.evals[eng.ply as usize - 2];

    // pruning
    if !pv_node && !pos.check && beta.abs() < Score::MATE {
        // reverse futility pruning
        if depth <= 8 && eval >= beta + 120 * depth / (1 + i32::from(improving)) { return eval }

        // razoring
        if depth <= 2 && eval + 400 * depth < alpha {
            let qeval = qs(pos, alpha, beta);
            if qeval < alpha { return qeval }
        }

        // null move pruning
        if null && depth >= 3 && pos.phase > 2 && eval >= beta {
            let mut new = *pos;
            let r = 3 + depth / 3;
            eng.push(hash);
            new.c = !new.c;
            new.enp_sq = 0;
            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false);
            eng.pop();
            if nw >= Score::MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    // internal iterative reduction
    if depth >= 4 && !tt_hit { depth -= 1 }

    // generating and scoring moves
    let mut moves = pos.movegen::<true>();
    let mut scores = [0; 252];
    let killers = eng.ktable[eng.ply as usize];
    for (i, &mov) in moves.list[..moves.len].iter().enumerate() {
        scores[i] = if mov == tt_move { MoveScore::HASH }
            else if mov.flag == Flag::ENP { 2 * MoveScore::HISTORY_MAX }
            else if mov.flag & 4 > 0 { mvv_lva(mov, pos) }
            else if mov.flag & 8 > 0 { MoveScore::PROMO + i32::from(mov.flag & 7) }
            else if killers.contains(&mov) { MoveScore::KILLER }
            else { eng.htable[usize::from(pos.c)][usize::from(mov.pc - 2)][usize::from(mov.to)] };
    }

    // stuff for going through moves
    let (mut legal, mut bound) = (0, Bound::UPPER);
    let (mut best_score, mut best_move) = (-Score::MAX, tt_move);
    let mut quiets_tried = MoveList::default();
    let can_lmr = depth > 1 && eng.ply > 0 && !pos.check;
    let lmr_base = (depth as f64).ln() / 2.67;

    eng.push(hash);
    while let Some((mov, ms)) = moves.pick(&mut scores) {
        let mut new = *pos;

        // skip move if not legal
        if new.make(mov) { continue }

        // update stuff
        new.check = new.in_check();
        eng.nodes += 1;
        legal += 1;
        if mov.flag < Flag::CAP {
            quiets_tried.list[quiets_tried.len] = mov;
            quiets_tried.len += 1;
        }

        // reductions
        let reduce = if can_lmr && ms < MoveScore::KILLER {
            // late move reductions - Viridithas values used
            let mut r = (0.77 + lmr_base * (legal as f64).ln()) as i32;

            // reduce pv nodes less
            r -= i32::from(pv_node);

            // reduce checks less
            r -= i32::from(new.check);

            // reduce passed pawn moves less
            let passed = usize::from(mov.pc) == Piece::PAWN
                && SPANS[usize::from(pos.c)][usize::from(mov.from)] & pos.bb[Piece::PAWN] & pos.bb[usize::from(!pos.c)] == 0;
            r -= i32::from(passed);

            // don't accidentally extend
            r.max(0)
        } else { 0 };

        // pvs
        let score = if legal == 1 {
            -pvs(&new, eng, -beta, -alpha, depth - 1, false)
        } else {
            let zw = -pvs(&new, eng, -alpha - 1, -alpha, depth - 1 - reduce, true);
            // re-search if fails high
            if zw > alpha && (pv_node || reduce > 0) {
                -pvs(&new, eng, -beta, -alpha, depth - 1, false)
            } else { zw }
        };

        // new best move
        if score <= best_score { continue }
        best_score = score;
        best_move = mov;

        // update pv line
        if pv_node {
            let sub_line = eng.pv_table[eng.ply as usize];
            let line = &mut eng.pv_table[eng.ply as usize - 1];
            line.len = 1 + sub_line.len;
            line.list[0] = mov;
            line.list[1..=sub_line.len].copy_from_slice(&sub_line.list[..sub_line.len]);
        }

        // improve alpha
        if score <= alpha { continue }
        alpha = score;
        bound = Bound::EXACT;

        // beta cutoff
        if score < beta { continue }
        bound = Bound::LOWER;

        // quiet cutoffs pushed to tables
        if mov.flag >= Flag::CAP || eng.abort { break }
        eng.push_killer(mov);
        let bonus = (16 * depth.pow(2)).min(1200);
        eng.push_history(mov, pos.c, bonus);
        for &quiet in &quiets_tried.list[..quiets_tried.len - 1] {
            eng.push_history(quiet, pos.c, -bonus / 2)
        }

        break
    }
    eng.pop();

    // end of node shenanigans
    if eng.abort { return 0 }
    if eng.ply == 0 { eng.best_move = best_move }
    if legal == 0 { return i32::from(pos.check) * (eng.ply - Score::MAX) }
    eng.push_tt(hash, best_move, depth, bound, best_score);
    best_score
}