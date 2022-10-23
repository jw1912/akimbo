use std::{ptr, mem, cmp::{min, max}, time::Instant};
use super::consts::*;
use super::position::*;
use super::hash::*;
use super::movegen::*;
use super::eval::*;
use super::u16_to_uci;

pub static mut DEPTH: i8 = i8::MAX;
pub static mut TIME: u128 = 1000;
static mut NODES: u64 = 0;
static mut STOP: bool = true;
static mut SELDEPTH: i8 = 0;

macro_rules! is_capture {($m:expr) => {$m & 0b0100_0000_0000_0000 > 0}}
macro_rules! is_promotion {($m:expr) => {$m & 0b1000_0000_0000_0000 > 0}}
macro_rules! is_castling {($m:expr) => {$m & 0b1110_0000_0000_0000 == 0b0010_0000_0000_0000}}
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

fn score_move(m: u16, hash_move: u16, killers: [u16; KILLERS_PER_PLY]) -> i16 {
    if m == hash_move {
        HASH_MOVE
    } else if is_capture!(m) {
        mvv_lva(m)
    } else if is_promotion!(m) {
        let pc = (m >> 12) & 3;
        PROMOTIONS[pc as usize]
    } else if killers.contains(&m) {
        KILLER
    } else if is_castling!(m) {
        CASTLE
    } else {
        QUIET
    }
}

fn score_moves(moves: &MoveList, move_scores: &mut MoveScores, hash_move: u16, ply: i8) {
    let killers = unsafe {KT[ply as usize]};
    for i in move_scores.start_idx..moves.len {
        let m = moves.list[i]; 
        move_scores.push(score_move(m, hash_move, killers));
    }
}
fn score_captures(moves: &MoveList, move_scores: &mut MoveScores) {
    for i in move_scores.start_idx..moves.len {
        move_scores.push(mvv_lva(moves.list[i]));
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

#[allow(clippy::too_many_arguments)]
fn pvs<const PV: bool>(mut alpha: i16, mut beta: i16, mut depth: i8, ply: i8, pv: &mut Vec<u16>, in_check: bool, start_time: &Instant) -> i16 {
    // search aborting
    unsafe {
    if STOP { return 0 }
    if NODES & 2047 == 0 && start_time.elapsed().as_millis() >= TIME {
        STOP = true;
        return 0
    }
    SELDEPTH = max(SELDEPTH, ply);
    }
    // draw detection
    if is_draw_by_50() || is_draw_by_repetition(2 + (ply == 0) as u8) || is_draw_by_material() { return 0 }
    // mate distance pruning
    alpha = max(alpha, -MAX + ply as i16);
    beta = min(beta, MAX - ply as i16 - 1);
    if alpha >= beta { return alpha }
    // check extensions
    depth += in_check as i8;
    // qsearch at depth 0
    if depth <= 0 || ply == MAX_PLY { return quiesce(alpha, beta) }
    unsafe{NODES += 1}
    // probing hash table
    let mut hash_move = 0;
    let mut write_to_hash = true;
    if let Some(res) = tt_probe(unsafe{POS.state.zobrist}, ply) {
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
    score_moves(&moves, &mut move_scores, hash_move, ply);
    // going through moves
    let mut best_move = 0;
    let mut best_score = -MAX;
    let mut bound: u8 = Bound::UPPER;
    let mut count = 0;
    while let Some((m, m_score)) = get_next_move(&mut moves, &mut move_scores) {
        let invalid = do_move(m);
        if invalid { continue }
        count += 1;
        let mut sub_pv = Vec::new();
        let gives_check = is_in_check();
        // LMR
        let r = (!in_check && !gives_check && count > 1 && m_score < 300) as i8;
        // score move
        let score = if count == 1 {
            -pvs::<PV>(-beta, -alpha, depth - 1, ply + 1, &mut sub_pv, gives_check, start_time)
        } else {
            let null_window_score = -pvs::<false>(-alpha - 1, -alpha, depth - 1 - r, ply + 1, &mut sub_pv, gives_check, start_time);
            if (null_window_score < beta || r > 0) && null_window_score > alpha {
                -pvs::<PV>(-beta, -alpha, depth - 1, ply + 1, &mut sub_pv, gives_check, start_time)
            } else { null_window_score }
        };
        undo_move();
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
                    if m & 0b0100_0000_0000_0000 == 0 { kt_push(m, ply) }
                    bound = Bound::LOWER;
                    break 
                }
            } 
        }
    }
    if count == 0 { return (in_check as i16) * (-MAX + ply as i16) }
    if write_to_hash { tt_push(unsafe{POS.state.zobrist}, best_move, depth, bound, best_score, ply) }
    best_score
}

fn quiesce(mut alpha: i16, beta: i16) -> i16 {
    unsafe{NODES += 1}
    let stand_pat = eval();
    if stand_pat >= beta { return beta }
    if stand_pat < alpha - 850 { return alpha }
    if alpha < stand_pat { alpha = stand_pat }
    let mut captures = MoveList::default();
    let mut scores = MoveScores::default();
    gen_moves::<Captures>(&mut captures);
    score_captures(&captures, &mut scores);
    while let Some((m, _)) = get_next_move(&mut captures, &mut scores) {
        let invalid = do_move(m);
        if invalid { continue }
        let score = -quiesce(-beta, -alpha);
        undo_move();
        if score >= beta { return beta }
        if score > alpha { alpha = score }
    }
    alpha
}

pub fn go() {
    unsafe {
        NODES = 0;
        SELDEPTH = 0;
        STOP = false;
    }
    let mut best_move = 0;
    let now = Instant::now();
    for d in 0..unsafe{DEPTH} {
        let mut pv = Vec::new();
        let in_check = is_in_check();
        let score = pvs::<true>(-MAX, MAX, d + 1, 0, &mut pv, in_check, &now);
        if unsafe{STOP} { break }
        let t = now.elapsed().as_millis();
        if t >= unsafe{TIME} { break }
        if !pv.is_empty() { best_move = pv[0] }
        let (stype, sval) = match is_mate_score!(score) {
            true => ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2), 
            false => ("cp", score)
        };
        let nps = ((unsafe{NODES} as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str = pv.iter().map(u16_to_uci).collect::<String>();
        println!("info depth {} seldepth {} score {} {} time {} nodes {} nps {} hashfull {} pv {}", d + 1, unsafe{SELDEPTH}, stype, sval, t, unsafe{NODES}, nps, hashfull(), pv_str);
        if is_mate_score!(score) { break }
    }
    unsafe {
        DEPTH = i8::MAX;
        TIME = 1000;
    }
    println!("bestmove {}", u16_to_uci(&best_move)); 
    kt_age();

}