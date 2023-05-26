use std::{sync::atomic::{AtomicU64, Ordering::Relaxed}, time::Instant};
use super::{util::{Bound, Flag, Score}, position::{Move, Position}};

static QNODES: AtomicU64 = AtomicU64::new(0);

fn mvv_lva(mov: Move, pos: &Position) -> i16 {
    Score::MVV_LVA * pos.get_pc(1 << mov.to) as i16 - mov.pc as i16
}

#[derive(Clone, Copy, Default)]
struct HashEntry {
    key: u16,
    best_move: u16,
    score: i16,
    depth: i8,
    bound: u8,
}

pub struct Engine {
    timing: Instant,
    pub max_time: u128,
    tt: Vec<HashEntry>,
    tt_age: u8,
    pub htable: Box<[[[i64; 64]; 6]; 2]>,
    pub hmax: i64,
    ktable: Box<[[Move; 2]; 96]>,
    pub stack: Vec<u64>,
    nodes: u64,
    ply: i16,
    abort: bool,
    best_move: Move,
    lines: Box<[Vec<Move>; 96]>,
}

impl Default for Engine {
    fn default() -> Self {
        const VEC: Vec<Move> = Vec::new();
        Self {
            timing: Instant::now(),
            max_time: Default::default(),
            tt: Default::default(),
            tt_age: Default::default(),
            htable: Box::new([[[0; 64]; 6]; 2]),
            hmax: 1,
            ktable: Box::new([[Move::default(); 2]; 96]),
            stack: Default::default(),
            nodes: Default::default(),
            ply: Default::default(),
            abort: Default::default(),
            best_move: Default::default(),
            lines: Box::new([VEC; 96]),
        }
    }
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

    pub fn resize_tt(&mut self, size: usize) {
        self.tt = vec![HashEntry::default(); 1 << (80 - (size as u64).leading_zeros())];
        self.tt_age = 0;
    }

    pub fn clear_tt(&mut self) {
        self.tt.iter_mut().for_each(|entry| *entry = HashEntry::default());
        self.tt_age = 0;
    }

    fn push_tt(&mut self, hash: u64, mov: Move, depth: i8, bound: u8, mut score: i16) {
        let (key, idx) = ((hash >> 48) as u16, (hash as usize) & (self.tt.len() - 1));
        let entry = &mut self.tt[idx];

        // replacement scheme
        let diff = self.tt_age - (entry.bound >> 2);
        if self.ply > 0 && key == entry.key && depth as u8 + 2 * diff < entry.depth as u8  { return }

        // replace entry
        score += if score.abs() > Score::MATE {score.signum() * self.ply} else {0};
        let best_move = u16::from(mov.from) << 6 | u16::from(mov.to) | u16::from(mov.flag) << 12;
        *entry = HashEntry { key, best_move, score, depth, bound: (self.tt_age << 2) | bound };
    }

    fn probe_tt(&self, hash: u64) -> Option<HashEntry> {
        let mut entry = self.tt[(hash as usize) & (self.tt.len() - 1)];
        if entry.key != (hash >> 48) as u16 { return None }
        entry.score -= if entry.score.abs() > Score::MATE {entry.score.signum() * self.ply} else {0};
        Some(entry)
    }

    fn push_killer(&mut self, m: Move) {
        let ply = self.ply as usize - 1;
        self.ktable[ply][1] = self.ktable[ply][0];
        self.ktable[ply][0] = m;
    }

    fn age_history(&mut self) {
        self.hmax = 1.max(self.hmax / 64);
        for side in self.htable.iter_mut() {
            for pc in side.iter_mut() {
                pc.iter_mut().for_each(|sq| *sq /= 64);
            }
        }
    }

    fn push_history(&mut self, mov: Move, side: bool, depth: i8) {
        let entry = &mut self.htable[usize::from(side)][usize::from(mov.pc - 2)][usize::from(mov.to)];
        *entry += i64::from(depth).pow(2);
        self.hmax = self.hmax.max(*entry);
    }

    fn score_history(&self, mov: Move, side: bool) -> i16 {
        let entry = self.htable[usize::from(side)][usize::from(mov.pc - 2)][usize::from(mov.to)];
        ((Score::MVV_LVA as i64 * entry + self.hmax - 1) / self.hmax) as i16
    }
}

pub fn go(start: &Position, eng: &mut Engine) {
    // reset engine
    *eng.ktable = [[Move::default(); 2]; 96];
    eng.age_history();
    eng.timing = Instant::now();
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
        let eval = pvs(&pos, eng, -Score::MAX, Score::MAX, d, false);
        if eng.abort { break }
        best = eng.best_move.to_uci();

        // UCI output
        let score = if eval.abs() >= Score::MATE {
            format!("score mate {}", if eval < 0 {eval.abs() - Score::MAX} else {Score::MAX - eval + 1} / 2)
        } else {format!("score cp {eval}")};
        let t = eng.timing.elapsed().as_millis();
        let nodes = eng.nodes + QNODES.load(Relaxed);
        let nps = (1000.0 * nodes as f64 / t as f64) as u32;
        let pv = eng.lines[0].iter().map(|mov| mov.to_uci()).collect::<String>();
        println!("info depth {d} {score} time {t} nodes {nodes} nps {nps:.0} pv {pv}");
    }
    eng.tt_age = 63.min(eng.tt_age + 1);
    println!("bestmove {best}");
}

fn qs(pos: &Position, mut alpha: i16, beta: i16) -> i16 {
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

fn pvs(pos: &Position, eng: &mut Engine, mut alpha: i16, mut beta: i16, mut depth: i8, null: bool) -> i16 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.elapsed().as_millis() >= eng.max_time {
        eng.abort = true;
        return 0
    }

    eng.lines[eng.ply as usize].clear();
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
    if depth <= 0 || eng.ply == 96 { return qs(pos, alpha, beta) }

    // probing hash table
    let pv_node = beta > alpha + 1;
    let mut best_move = Move::default();
    if let Some(res) = eng.probe_tt(hash) {
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
            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false);
            eng.pop();
            if nw >= Score::MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    // internal iterative reduction
    if depth >= 4 && best_move == Move::default() { depth -= 1 }

    // generating and scoring moves
    let mut moves = pos.movegen::<true>();
    let mut scores = [0; 252];
    let killers = eng.ktable[eng.ply as usize];
    for (i, &mov) in moves.list[..moves.len].iter().enumerate() {
        scores[i] = if mov == best_move { Score::MAX }
            else if mov.flag == Flag::ENP { 2 * Score::MVV_LVA }
            else if mov.flag & 4 > 0 { mvv_lva(mov, pos) }
            else if mov.flag & 8 > 0 { Score::PROMO + i16::from(mov.flag & 7) }
            else if killers.contains(&mov) { Score::KILLER }
            else { eng.score_history(mov, pos.c) };
    }

    // stuff for going through moves
    let (mut legal, mut eval, mut bound) = (0, -Score::MAX, Bound::UPPER);
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
        if pv_node {
            let sub_line = eng.lines[eng.ply as usize].clone();
            let line = &mut eng.lines[eng.ply as usize - 1];
            line.clear();
            line.push(mov);
            line.extend_from_slice(&sub_line);
        }

        if score <= alpha { continue }
        alpha = score;
        bound = Bound::EXACT;

        if score < beta { continue }
        bound = Bound::LOWER;

        // quiet cutoffs pushed to tables
        if mov.flag >= Flag::CAP || eng.abort { break }
        eng.push_killer(mov);
        eng.push_history(mov, pos.c, depth);

        break
    }
    eng.pop();

    // end of node shenanigans
    if eng.abort { return 0 }
    if eng.ply == 0 { eng.best_move = best_move }
    if legal == 0 { return i16::from(pos.check) * (-Score::MAX + eng.ply) }
    eng.push_tt(hash, best_move, depth, bound, eval);
    eval
}