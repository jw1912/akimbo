use std::{ptr, mem, cmp::{min, max}};
use super::consts::*;
use super::position::*;
use super::hash::*;
use super::movegen::*;
use super::eval::*;

static mut DEPTH: i8 = 1;

macro_rules! is_capture {($m:expr) => {$m & 0b0100_0000_0000_0000 > 0}}
macro_rules! is_mate_score {($score:expr) => {$score >= MATE_THRESHOLD || $score <= -MATE_THRESHOLD}}

struct MoveScores {
    list: [i16; 255],
    len: usize,
    start_idx: usize,
}
impl Default for MoveScores {
    fn default() -> Self {
        Self {
            list: unsafe {
                #[allow(clippy::uninit_assumed_init)]
                mem::MaybeUninit::uninit().assume_init()
            },
            len: 0,
            start_idx: 0,
        } 
    }
}
impl MoveScores {
    #[inline(always)]
    fn push(&mut self, m: i16) {
        self.list[self.len] = m;
        self.len += 1;
    }
    #[inline(always)]
    fn swap_unchecked(&mut self, i: usize, j: usize) {
        let ptr = self.list.as_mut_ptr();
        unsafe {
            ptr::swap(ptr.add(i), ptr.add(j));
        }
    }
}

fn mvv_lva( m: u16) -> i16 {
    let from_idx = (m >> 6) & 0b111111;
    let to_idx = m & 0b111111;
    let moved_pc = unsafe{POS.squares[from_idx as usize]} as usize;
    let captured_pc = unsafe{POS.squares[to_idx as usize]} as usize;
    MVV_LVA[captured_pc][moved_pc]
}

fn score_move(m: u16, hash_move: u16) -> i16 {
    if m == hash_move {
        HASH_MOVE
    } else if is_capture!(m) {
        mvv_lva(m)
    } else {
        QUIET
    }
}

fn score_moves(moves: &MoveList, move_scores: &mut MoveScores, hash_move: u16) {
    for i in move_scores.start_idx..moves.len {
        let m = moves.list[i]; 
        move_scores.push(score_move(m, hash_move));
    }
}

fn get_next_move(moves: &mut MoveList, move_scores: &mut MoveScores) -> Option<(u16, i16)> {
    let m_idx = move_scores.start_idx;
    if m_idx == move_scores.len {
        return None
    }
    let mut best_idx = 0;
    let mut best_score = i16::MIN;
    for i in m_idx..move_scores.len {
        let score = move_scores.list[i];
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }
    let m = moves.list[best_idx];
    move_scores.swap_unchecked(best_idx, m_idx);
    moves.swap_unchecked(best_idx, m_idx);
    move_scores.start_idx += 1;
    Some((m, best_score))
}

fn pvs<const PV: bool>(mut alpha: i16, mut beta: i16, depth: i8, ply: i8, pv: &mut Vec<u16>, in_check: bool) -> i16 {
    // mate distance pruning
    alpha = max(alpha, -MAX + ply as i16);
    beta = min(beta, MAX - ply as i16 - 1);
    if alpha >= beta { 
        return alpha 
    }
    // qsearch at depth 0
    if depth <= 0 || ply == MAX_PLY { return quiesce(alpha, beta) }
    // probing hash table
    let zobrist = zobrist::calc();
    let mut hash_move = 0;
    let mut write_to_hash = true;
    if let Some(res) = tt_probe(zobrist, ply) {
        write_to_hash = depth > res.depth;
        hash_move = res.best_move;
        if ply > 0 && res.depth >= depth && unsafe{POS.state.halfmove_clock} <= 90 {
            match res.bound {
                Bound::EXACT => { if !PV { return res.score } },
                Bound::LOWER => { if res.score >= beta { return beta } },
                Bound::UPPER => { if res.score <= alpha { return alpha } },
                _ => ()
            }
        }
    }
    // generating and scoring moves
    let mut moves = MoveList::default();
    let mut move_scores = MoveScores::default();
    gen_moves::<All>(&mut moves);
    score_moves(&moves, &mut move_scores, hash_move);
    // going through moves
    let mut best_move = 0;
    let mut best_score = -MAX;
    let mut bound: u8 = Bound::UPPER;
    let mut count = 0;
    while let Some((m, m_score)) = get_next_move(&mut moves, &mut move_scores) {
        let ctx = do_move(m);
        if ctx.invalid { continue }
        count += 1;
        let mut sub_pv = Vec::new();
        let gives_check = is_in_check();
        // LMR
        let r = (!in_check && !gives_check && count > 1 && m_score <= 300) as i8;
        // score move
        let score = if count == 1 {
            -pvs::<PV>(-beta, -alpha, depth - 1, ply + 1, &mut sub_pv, gives_check)
        } else {
            let null_window_score = -pvs::<false>(-alpha - 1, -alpha, depth - 1 - r, ply + 1, &mut sub_pv, gives_check);
            if (null_window_score < beta || r > 0) && null_window_score > alpha {
                -pvs::<PV>(-beta, -alpha, depth - 1, ply + 1, &mut sub_pv, gives_check)
            } else { null_window_score }
        };
        undo_move(ctx);
        if score > best_score {
            best_score = score;
            best_move = m;
            if score > alpha { 
                bound = Bound::EXACT;
                alpha = score;
                pv.clear();
                pv.push(m);
                pv.append(&mut sub_pv);
                if score >= beta { 
                    bound = Bound::LOWER;
                    break 
                }
            } 
        }
    }
    if count == 0 { return (in_check as i16) * (-MAX + ply as i16) }
    if write_to_hash { tt_push(zobrist, best_move, depth, bound, best_score, ply) }
    best_score
}

fn quiesce(mut alpha: i16, beta: i16) -> i16 {
    let stand_pat = eval();
    if stand_pat >= beta { return beta }
    if stand_pat < alpha - 850 { return alpha }
    if alpha < stand_pat { alpha = stand_pat }
    let mut captures = MoveList::default();
    let mut scores = MoveScores::default();
    gen_moves::<Captures>(&mut captures);
    score_moves(&captures, &mut scores, 0);
    while let Some((m, _)) = get_next_move(&mut captures, &mut scores) {
        let ctx = do_move(m);
        if ctx.invalid { continue }
        let score = -quiesce(-beta, -alpha);
        undo_move(ctx);
        if score >= beta { return beta }
        if score > alpha { alpha = score }
    }
    alpha
}

pub fn go() -> u16 {
    let mut best_move = 0;
    for d in 0..unsafe{DEPTH} {
        let mut pv = Vec::new();
        let in_check = is_in_check();
        let score = pvs::<true>(-MAX, MAX, d + 1, 0, &mut pv, in_check);
        if !pv.is_empty() { best_move = pv[0] }
        let (stype, sval) = match is_mate_score!(score) {
            true => ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2), 
            false => ("cp", score)
        };
        println!("info depth {} score {} {} pv {}", d + 1, stype, sval, best_move);
        if is_mate_score!(score) { break }
    }
    best_move
}