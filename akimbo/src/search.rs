use std::time::Instant;
use super::{util::{Bound, Flag, MoveScore, Piece, Score}, position::{Move, MoveList, Position}};

fn mvv_lva(mov: Move, pos: &Position) -> i32 {
    8 * pos.get_pc(1 << mov.to) as i32 - mov.pc as i32
}

#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    key: u16,
    best_move: u16,
    score: i16,
    depth: i8,
    bound: u8,
}

type PlyInfo = ([Move; 2], i32, Move, MoveList, i32);
type History = (i32, Move);

pub struct Engine {
    // search control
    pub timing: Instant,
    pub max_time: u128,
    pub max_nodes: u64,
    pub abort: bool,
    pub mloop: bool,

    // tables
    pub tt: Vec<HashEntry>,
    pub tt_age: u8,
    pub htable: Box<[[[History; 64]; 8]; 2]>,
    pub plied: Box<[PlyInfo; 96]>,
    pub ntable: Box<[[u64; 64]; 64]>,
    pub stack: Vec<u64>,

    // uci output
    pub nodes: u64,
    pub qnodes: u64,
    pub ply: i32,
    pub best_move: Move,
    pub seldepth: i32,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            timing: Instant::now(), max_time: 0, abort: false, max_nodes: u64::MAX, mloop: true,
            tt: Vec::new(), tt_age: 0,
            htable: Box::new([[[(0, Move::NULL); 64]; 8]; 2]),
            plied: Box::new([([Move::NULL; 2], 0, Move::NULL, MoveList::ZEROED, 0); 96]),
            ntable: Box::new([[0; 64]; 64]),
            stack: Vec::with_capacity(96),
            nodes: 0, qnodes: 0, ply: 0, best_move: Move::NULL, seldepth: 0,
        }
    }
}

impl Engine {
    pub fn repetition(&self, pos: &Position, curr_hash: u64, root: bool) -> bool {
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
        self.plied[self.ply as usize].4 = 0;
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

    fn push_tt(&mut self, hash: u64, mov: Move, depth: i8, bound: u8, mut score: i32) {
        let (key, idx) = ((hash >> 48) as u16, (hash as usize) & (self.tt.len() - 1));
        let entry = &mut self.tt[idx];

        // replacement scheme
        let diff = self.tt_age - (entry.bound >> 2);
        if self.ply > 0 && key == entry.key && depth as u8 + 2 * diff < entry.depth as u8  { return }

        // replace entry
        score += if score.abs() > Score::MATE {score.signum() * self.ply} else {0};
        let best_move = u16::from(mov.from) << 6 | u16::from(mov.to) | u16::from(mov.flag) << 12;
        *entry = HashEntry { key, best_move, score: score as i16, depth, bound: (self.tt_age << 2) | bound };
    }

    fn probe_tt(&self, hash: u64) -> Option<HashEntry> {
        let mut entry = self.tt[(hash as usize) & (self.tt.len() - 1)];
        if entry.key != (hash >> 48) as u16 { return None }
        entry.score -= if entry.score.abs() > Score::MATE as i16 {entry.score.signum() * self.ply as i16} else {0};
        Some(entry)
    }

    fn push_killer(&mut self, m: Move) {
        let ply = self.ply as usize - 1;
        self.plied[ply].0[1] = self.plied[ply].0[0];
        self.plied[ply].0[0] = m;
    }

    fn push_history(&mut self, mov: Move, side: bool, bonus: i32) {
        let entry = &mut self.htable[usize::from(side)][usize::from(mov.pc)][usize::from(mov.to)];
        entry.0 += bonus - entry.0 * bonus.abs() / MoveScore::HISTORY_MAX
    }
}

pub fn go(start: &Position, eng: &mut Engine, report: bool, max_depth: i32, soft_bound: f64, soft_nodes: u64) -> (Move, i32) {
    // reset engine
    *eng.ntable = [[0; 64]; 64];
    eng.plied.iter_mut().for_each(|x| x.0 = [Move::NULL; 2]);
    eng.htable.iter_mut().flatten().flatten().for_each(|x| *x = (x.0 / 2, Move::NULL));
    eng.timing = Instant::now();
    eng.nodes = 0;
    eng.qnodes = 0;
    eng.ply = 0;
    eng.best_move = Move::NULL;
    eng.abort = false;
    eng.seldepth = 0;

    let mut best_move = Move::NULL;
    let mut pos = *start;
    let (mut eval, mut score) = (0, 0);
    pos.check = pos.in_check();

    // iterative deepening loop
    for d in 1..=max_depth {
        if eng.nodes + eng.qnodes > soft_nodes { break }

        eval = if d < 7 {
            pvs(&pos, eng, -Score::MAX, Score::MAX, d, false, Move::NULL)
        } else { aspiration(&pos, eng, eval, d, &mut best_move, Move::NULL) };

        if eng.abort { break }
        best_move = eng.best_move;
        score = eval;

        let nodes = eng.nodes + eng.qnodes;

        // UCI output
        if report {
            let score = if eval.abs() >= Score::MATE {
                format!("score mate {}", if eval < 0 {eval.abs() - Score::MAX} else {Score::MAX - eval + 1} / 2)
            } else {format!("score cp {eval}")};
            let t = eng.timing.elapsed().as_millis();
            let nps = (1000.0 * nodes as f64 / t as f64) as u32;
            let pv_line = &eng.plied[0].3;
            let pv = pv_line.list.iter().take(pv_line.len).map(|mov| format!("{} ", mov.to_uci())).collect::<String>();
            println!("info depth {d} seldepth {} {score} time {t} nodes {nodes} nps {nps:.0} pv {pv}", eng.seldepth);
        }

        let frac = eng.ntable[usize::from(best_move.from)][usize::from(best_move.to)] as f64 / nodes as f64;
        if eng.timing.elapsed().as_millis() as f64 >= soft_bound * if d > 8 {(1.5 - frac) * 1.35} else {1.0} { break }
    }
    eng.tt_age = 63.min(eng.tt_age + 1);
    (best_move, score)
}

fn aspiration(pos: &Position, eng: &mut Engine, mut score: i32, max_depth: i32, best_move: &mut Move, prev: Move) -> i32 {
    let mut delta = 25;
    let mut alpha = (-Score::MAX).max(score - delta);
    let mut beta = Score::MAX.min(score + delta);
    let mut depth = max_depth;

    loop {
        score = pvs(pos, eng, alpha, beta, depth, false, prev);
        if eng.abort { return 0 }

        if score <= alpha {
            beta = (alpha + beta) / 2;
            alpha = (-Score::MAX).max(alpha - delta);
            depth = max_depth;
        } else if score >= beta {
            beta = Score::MAX.min(beta + delta);
            *best_move = eng.best_move;
            depth -= 1;
        } else {
            return score
        }

        delta *= 2;
    }
}

fn qs(pos: &Position, eng: &mut Engine, mut alpha: i32, beta: i32) -> i32 {
    eng.seldepth = eng.seldepth.max(eng.ply);
    let mut eval = pos.eval();
    if eval >= beta { return eval }
    alpha = alpha.max(eval);

    // probe hash table for cutoff
    let hash = pos.hash();
    if let Some(res) = eng.probe_tt(hash) {
        let tt_score = i32::from(res.score);
        if match res.bound & 3 {
            Bound::LOWER => tt_score >= beta,
            Bound::UPPER => tt_score <= alpha,
            _ => true,
        } { return tt_score }
    }

    let mut caps = pos.movegen::<false>();
    let mut scores = [0; 252];
    caps.list.iter().enumerate().take(caps.len).for_each(|(i, &cap)| scores[i] = mvv_lva(cap, pos));

    eng.ply += 1;
    let mut bm = Move::NULL;
    while let Some((mov, _)) = caps.pick(&mut scores) {
        // static exchange eval pruning
        if !pos.see(mov, 1) { continue }

        let mut new = *pos;
        if new.make(mov) { continue }
        eng.qnodes += 1;

        let score = -qs(&new, eng, -beta, -alpha);

        if score <= eval { continue }
        eval = score;
        bm = mov;

        if eval >= beta { break }
        alpha = alpha.max(eval);
    }
    eng.ply -= 1;

    eng.push_tt(hash, bm, 0, if eval >= beta {Bound::LOWER} else {Bound::UPPER}, eval);
    eval
}

fn pvs(pos: &Position, eng: &mut Engine, mut alpha: i32, mut beta: i32, mut depth: i32, null: bool, prev: Move) -> i32 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && (eng.timing.elapsed().as_millis() >= eng.max_time || eng.nodes + eng.qnodes >= eng.max_nodes) {
        eng.abort = true;
        return 0
    }

    let hash = pos.hash();

    // clear pv line
    eng.plied[eng.ply as usize].3.len = 0;

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
    if depth <= 0 || eng.ply == 95 { return qs(pos, eng, alpha, beta) }

    // probing hash table
    let pv_node = beta > alpha + 1;
    let s_mov = eng.plied[eng.ply as usize].2;
    let singular = s_mov != Move::NULL;
    let mut eval = pos.eval();
    let mut tt_move = Move::NULL;
    let mut tt_score = -Score::MAX;
    let mut try_singular = !singular && depth >= 8 && eng.ply > 0;
    if let Some(res) = eng.probe_tt(hash) {
        let bound = res.bound & 3;
        tt_move = Move::from_short(res.best_move, pos);
        tt_score = i32::from(res.score);
        try_singular &= i32::from(res.depth) >= depth - 3 && bound != Bound::UPPER && tt_score.abs() < Score::MATE;

        // tt cutoffs
        if !singular && !pv_node && depth <= i32::from(res.depth) && match bound {
            Bound::LOWER => tt_score >= beta,
            Bound::UPPER => tt_score <= alpha,
            _ => true,
        } { return tt_score }

        // use tt score instead of static eval
        if !((eval > tt_score && bound == Bound::LOWER) || (eval < tt_score && bound == Bound::UPPER)) {
            eval = tt_score;
        }
    }

    // improving heuristic
    eng.plied[eng.ply as usize].1 = eval;
    let improving = eng.ply > 1 && eval > eng.plied[eng.ply as usize - 2].1;

    // pruning
    let mut can_prune = !pv_node && !pos.check;
    if can_prune && beta.abs() < Score::MATE {
        // reverse futility pruning
        if depth <= 8 && eval >= beta + 80 * depth / if improving {2} else {1} { return eval }

        // razoring
        if depth <= 2 && eval + 400 * depth < alpha {
            let qeval = qs(pos, eng, alpha, beta);
            if qeval < alpha { return qeval }
        }

        // null move pruning
        if null && depth >= 3 && pos.phase > 2 && eval >= beta {
            let mut new = *pos;
            let r = 3 + depth / 3;
            eng.push(hash);
            new.c = !new.c;
            new.enp_sq = 0;
            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false, Move::NULL);
            eng.pop();
            if nw >= Score::MATE { return beta }
            if nw >= beta { return nw }
        }
    }

    // internal iterative reduction
    if depth >= 4 && tt_move == Move::NULL { depth -= 1 }

    // generating and scoring moves
    let mut moves = pos.movegen::<true>();
    let mut scores = [0; 252];
    let killers = eng.plied[eng.ply as usize].0;
    let counter_mov = if prev != Move::NULL {
        eng.htable[usize::from(pos.c)][usize::from(prev.pc)][usize::from(prev.to)].1
    } else {Move::NULL};
    moves.list[..moves.len].iter().enumerate().for_each(|(i, &mov)|
        scores[i] = if mov == tt_move { MoveScore::HASH }
            else if mov.flag == Flag::ENP { MoveScore::CAPTURE + 16 }
            else if mov.flag & 4 > 0 { MoveScore::CAPTURE * i32::from(pos.see(mov, 0)) + mvv_lva(mov, pos) }
            else if mov.flag & 8 > 0 { MoveScore::PROMO + i32::from(mov.flag & 7) }
            else if killers.contains(&mov) { MoveScore::KILLER + 1 }
            else if mov == counter_mov { MoveScore::KILLER }
            else { eng.htable[usize::from(pos.c)][usize::from(mov.pc)][usize::from(mov.to)].0 }
    );

    // stuff for going through moves
    let (mut legal, mut bound) = (0, Bound::UPPER);
    let (mut best_score, mut best_move) = (-Score::MAX, tt_move);
    let mut quiets_tried = MoveList::ZEROED;
    let can_lmr = depth > 1 && eng.ply > 0 && !pos.check;
    let lmr_base = (depth as f64).ln() / 2.67;
    can_prune &= eng.mloop;

    eng.push(hash);
    while let Some((mov, ms)) = moves.pick(&mut scores) {
        // move is singular in a singular search
        if mov == s_mov { continue }

        // pre-move pruning
        if can_prune && best_score.abs() < Score::MATE {
            // late move pruning
            if ms < MoveScore::KILLER && legal > 2 + depth * depth / if improving {1} else {2} { break }

            // static exchange eval pruning
            let margin = if mov.flag & Flag::CAP > 0 {-90} else {-50};
            if depth < 7 && ms < MoveScore::CAPTURE && !pos.see(mov, margin * depth) { continue }
        }

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

        // singular extensions
        let ext = if try_singular && mov == tt_move {
            let s_beta = tt_score - depth * 2;
            eng.pop();
            eng.plied[eng.ply as usize].2 = mov;
            let ret = pvs(pos, eng, s_beta - 1, s_beta, (depth - 1) / 2, false, prev);
            eng.plied[eng.ply as usize].2 = Move::NULL;
            eng.push(hash);
            if ret < s_beta {1} else {0}
        } else {0};

        // reductions
        let reduce = if can_lmr && ms < MoveScore::KILLER {
            // late move reductions - Viridithas values used
            let mut r = (0.77 + lmr_base * (legal as f64).ln()) as i32;

            // reduce pv nodes less
            r -= i32::from(pv_node);

            // reduce checks less
            r -= i32::from(new.check);

            // reduce passed pawn moves less
            r -= i32::from(usize::from(mov.pc) == Piece::PAWN && pos.is_passer(mov.from, usize::from(pos.c)));

            // reduce less if next ply had few fail highs
            r -= i32::from(eng.plied[eng.ply as usize].4 < 4);

            // don't accidentally extend
            r.max(0)
        } else { 0 };

        let pre_nodes = eng.nodes + eng.qnodes;

        // pvs
        let score = if legal == 1 {
            -pvs(&new, eng, -beta, -alpha, depth + ext - 1, false, mov)
        } else {
            let zw = -pvs(&new, eng, -alpha - 1, -alpha, depth - 1 - reduce, true, mov);
            if zw > alpha && (pv_node || reduce > 0) {
                -pvs(&new, eng, -beta, -alpha, depth - 1, false, mov)
            } else { zw }
        };

        if eng.ply == 1 { eng.ntable[usize::from(mov.from)][usize::from(mov.to)] += eng.nodes + eng.qnodes - pre_nodes }

        best_score = best_score.max(score);

        // improve alpha
        if score <= alpha { continue }
        best_move = mov;
        alpha = score;
        bound = Bound::EXACT;

        // update pv line
        if pv_node {
            let sub_line = eng.plied[eng.ply as usize].3;
            let line = &mut eng.plied[eng.ply as usize - 1].3;
            line.len = 1 + sub_line.len;
            line.list[0] = mov;
            line.list[1..=sub_line.len].copy_from_slice(&sub_line.list[..sub_line.len]);
        }

        // beta cutoff
        if score < beta { continue }
        bound = Bound::LOWER;
        eng.plied[eng.ply as usize - 1].4 += 1;

        // quiet cutoffs pushed to tables
        if mov.flag >= Flag::CAP || eng.abort { break }
        eng.push_killer(mov);
        let bonus = 1600.min(350 * (depth - 1));
        eng.push_history(mov, pos.c, bonus);
        for &quiet in &quiets_tried.list[..quiets_tried.len - 1] {
            eng.push_history(quiet, pos.c, -bonus)
        }
        eng.htable[usize::from(pos.c)][usize::from(prev.pc)][usize::from(prev.to)].1 = mov;

        break
    }
    eng.pop();

    // end of node shenanigans
    if eng.abort { return 0 }
    if eng.ply == 0 { eng.best_move = best_move }
    if legal == 0 { return i32::from(pos.check) * (eng.ply - Score::MAX) }
    eng.push_tt(hash, best_move, depth as i8, bound, best_score);
    best_score
}