use std::{cmp::{min, max}, time::Instant};
use super::consts::*;
use super::position::*;
use super::hash::*;
use super::movegen::*;
use super::eval::*;
use super::u16_to_uci;

// search info
pub static mut DEPTH: i8 = i8::MAX;
pub static mut TIME: u128 = 1000;
pub static mut PLY: i8 = 0;

// UCI info
static mut NODES: u64 = 0;
static mut STOP: bool = true;
static mut SELDEPTH: i8 = 0;
static mut PV_LINE: [u16; MAX_PLY as usize] = [0; MAX_PLY as usize];

macro_rules! is_capture {($m:expr) => {$m & 0b0100_0000_0000_0000 > 0}}
macro_rules! is_mate_score {($score:expr) => {$score.abs() >= MATE_THRESHOLD}}

fn mvv_lva(m: u16) -> u16 {
    let from_idx: u16 = (m >> 6) & 0b111111;
    let to_idx: u16 = m & 0b111111;
    let moved_pc: usize = unsafe{POS.squares[from_idx as usize]} as usize;
    let captured_pc: usize = unsafe{POS.squares[to_idx as usize]} as usize;
    MVV_LVA[captured_pc][moved_pc]
}

fn score_move(m: u16, hash_move: u16, killers: [u16; KILLERS_PER_PLY]) -> u16 {
    if m == hash_move {
        HASH_MOVE
    } else if is_capture!(m) {
        mvv_lva(m)
    } else if killers.contains(&m) {
        KILLER
    } else {
        QUIET
    }
}

fn score_moves(moves: &MoveList, move_scores: &mut MoveList, hash_move: u16, start_idx: usize) {
    let killers: [u16; KILLERS_PER_PLY] = unsafe{KT[PLY as usize]};
    for i in start_idx..moves.len {
        let m: u16 = moves.list[i]; 
        move_scores.push(score_move(m, hash_move, killers));
    }
}

fn score_captures(moves: &MoveList, move_scores: &mut MoveList, start_idx: usize) {
    for i in start_idx..moves.len {
        move_scores.push(mvv_lva(moves.list[i]));
    }
}

fn get_next_move(moves: &mut MoveList, move_scores: &mut MoveList, start_idx: &mut usize) -> Option<(u16, u16)> {
    let m_idx: usize = *start_idx;
    if m_idx == move_scores.len {return None}
    let mut best_idx: usize = m_idx;
    let mut best_score: u16 = 0;
    for i in m_idx..move_scores.len {
        let score: u16 = move_scores.list[i];
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }
    let m: u16 = moves.list[best_idx];
    move_scores.swap_unchecked(best_idx, m_idx);
    moves.swap_unchecked(best_idx, m_idx);
    *start_idx += 1;
    Some((m, best_score))
}

unsafe fn pvs(pv: bool, mut alpha: i16, mut beta: i16, mut depth: i8, in_check: bool, start_time: &Instant, allow_null: bool) -> i16 {
    // search aborting
    if STOP { return 0 }
    if NODES & 2047 == 0 && start_time.elapsed().as_millis() >= TIME {
        STOP = true;
        return 0
    }
    // draw detection
    if is_draw_by_50() || is_draw_by_repetition(2 + (PLY == 0) as u8) || is_draw_by_material() { return 0 }
    // mate distance pruning
    alpha = max(alpha, -MAX + PLY as i16);
    beta = min(beta, MAX - PLY as i16 - 1);
    if alpha >= beta { return alpha }
    // check extensions
    depth += in_check as i8;
    // qsearch at depth 0
    if depth <= 0 || PLY == MAX_PLY { 
        SELDEPTH = max(SELDEPTH, PLY);
        return quiesce(alpha, beta) 
    }
    NODES += 1;
    // probing hash table
    let mut hash_move: u16 = 0;
    let mut write_to_hash: bool = true;
    if let Some(res) = tt_probe(POS.state.zobrist) {
        write_to_hash = depth > res.depth;
        hash_move = res.best_move;
        if PLY > 0 && res.depth >= depth && POS.state.halfmove_clock <= 90 {
            match res.bound {
                Bound::EXACT => { if !pv { return res.score } },
                Bound::LOWER => { if res.score >= beta { return beta } },
                Bound::UPPER => { if res.score <= alpha { return alpha } },
                _ => ()
            }
        }
    }
    // reverse futility and null move pruning
    if !pv && !in_check && beta.abs() < MATE_THRESHOLD {
        let lazy_eval: i16 = lazy_eval();
        if depth <= 8 && lazy_eval >= beta + 120 * depth as i16 {
            return beta
        } else if allow_null && depth >= 3 && POS.state.phase >= 2 && lazy_eval >= beta {
            let ctx: (u16, u64) = do_null();
            let score: i16 = -pvs(false, -beta, -beta + 1, depth - 3, false, start_time, false);
            undo_null(ctx);
            if score >= beta {return beta}
        }
    }
    // generating and scoring moves
    let mut moves = MoveList::default();
    let mut move_scores = MoveList::default();
    let mut m_idx: usize = 0;
    gen_moves::<ALL>(&mut moves);
    score_moves(&moves, &mut move_scores, hash_move, m_idx);
    // going through moves
    PLY += 1;
    let mut best_move: u16 = 0;
    let mut best_score: i16 = -MAX;
    let mut bound: u8 = Bound::UPPER;
    let mut count: u16 = 0;
    while let Some((m, m_score)) = get_next_move(&mut moves, &mut move_scores, &mut m_idx) {
        let invalid: bool = do_move(m);
        if invalid { continue }
        count += 1;
        let gives_check = is_in_check();
        // LMR
        let r: i8 = (!in_check && !gives_check && count > 1 && m_score < 300) as i8;
        // score move
        let score: i16 = if count == 1 {
            -pvs(pv, -beta, -alpha, depth - 1, gives_check, start_time, false)
        } else {
            let null_window_score: i16 = -pvs(false, -alpha - 1, -alpha, depth - 1 - r, gives_check, start_time, true);
            if (null_window_score < beta || r > 0) && null_window_score > alpha {
                -pvs(pv, -beta, -alpha, depth - 1, gives_check, start_time, false)
            } else { null_window_score }
        };
        undo_move();
        if score > best_score {
            best_score = score;
            best_move = m;
            if score > alpha { 
                bound = Bound::EXACT;
                alpha = score;
                if pv { PV_LINE[PLY as usize - 1] = m }
                if score >= beta {
                    if m & 0b0100_0000_0000_0000 == 0 { kt_push(m) };
                    bound = Bound::LOWER;
                    break 
                }
            } 
        }
    }
    PLY -= 1;
    if count == 0 { return (in_check as i16) * (-MAX + PLY as i16) }
    if write_to_hash { tt_push(POS.state.zobrist, best_move, depth, bound, best_score) }
    best_score
}

pub fn quiesce(mut alpha: i16, beta: i16) -> i16 {
    unsafe {
    NODES += 1;
    let stand_pat = static_eval();
    if stand_pat >= beta { return beta }
    if stand_pat < alpha - 850 { return alpha }
    if alpha < stand_pat { alpha = stand_pat }
    let mut captures = MoveList::default();
    let mut scores = MoveList::default();
    let mut m_idx: usize = 0;
    gen_moves::<CAPTURES>(&mut captures);
    score_captures(&captures, &mut scores, m_idx);
    while let Some((m, _)) = get_next_move(&mut captures, &mut scores, &mut m_idx) {
        let invalid: bool = do_move(m);
        if invalid { continue }
        let score: i16 = -quiesce(-beta, -alpha);
        undo_move();
        if score >= beta { return beta }
        if score > alpha { alpha = score }
    }
    alpha}
}

pub fn go() {
    unsafe {
    NODES = 0;
    SELDEPTH = 0;
    STOP = false;
    let mut best_move: u16 = 0;
    let now = Instant::now();
    for d in 0..DEPTH {
        let in_check: bool = is_in_check();
        let score: i16 = pvs(true, -MAX, MAX, d + 1, in_check, &now, false);
        if STOP { break }
        let t: u128 = now.elapsed().as_millis();
        if t >= TIME { break }
        best_move = PV_LINE[0];
        let (stype, sval): (&str, i16) = match is_mate_score!(score) {
            true => ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2), 
            false => ("cp", score)
        };
        let nps: u32 = ((NODES as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str = PV_LINE[..(d as usize + 1)].iter().map(u16_to_uci).collect::<String>();
        println!("info depth {} seldepth {} score {} {} time {} nodes {} nps {} hashfull {} pv {}", d + 1, SELDEPTH, stype, sval, t, NODES, nps, hashfull(), pv_str);
        if is_mate_score!(score) { break }
    }
    DEPTH = i8::MAX;
    TIME = 1000;
    println!("bestmove {}", u16_to_uci(&best_move));
    kt_age();
    }
}