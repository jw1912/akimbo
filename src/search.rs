use super::{from, to, consts::*, position::*, tables::*, movegen::*, u16_to_uci};
use std::{cmp::{min, max}, time::Instant};

// Search parameters.
pub static mut DEPTH: i8 = i8::MAX;
pub static mut TIME: u128 = u128::MAX;

// Search statistics.
pub static mut PLY: i8 = 0;
pub static mut STOP: bool = true;
static mut NODES: u64 = 0;
static mut BEST_MOVE: u16 = 0;
static mut START_TIME: Option<Instant> = None;

#[inline(always)]
unsafe fn lazy_eval() -> i16 {
    let phase: i32 = min(POS.state.phase as i32, TPHASE);
    SIDE_FACTOR[POS.side_to_move] * ((phase * POS.state.mg as i32 + (TPHASE - phase) * POS.state.eg as i32) / TPHASE) as i16
}

fn mvv_lva(m: u16) -> u16 {
    let from_idx: usize = from!(m);
    let to_idx: usize = to!(m);
    let moved_pc: usize = unsafe{POS.squares[from_idx]} as usize;
    let captured_pc: usize = unsafe{POS.squares[to_idx]} as usize;
    MVV_LVA[captured_pc][moved_pc]
}

/// Scores a move.
/// 1. Hash move
/// 2. Captures, sorted by MVV-LVA
/// 3. Promotions
/// 4. Killer moves
/// 5. Quiet moves
fn score_move(m: u16, hash_move: u16, killers: [u16; 3]) -> u16 {
    if m == hash_move {
        HASH_MOVE
    } else if m & 0b0100_0000_0000_0000 > 0 {
        mvv_lva(m)
    } else if m & 0b1000_0000_0000_0000 > 0 {
        PROMOTION
    } else if killers.contains(&m) {
        KILLER
    } else {
        QUIET
    }
}

fn score_moves(moves: &MoveList, move_scores: &mut MoveList, hash_move: u16, start_idx: usize) {
    let killers: [u16; 3] = unsafe{KT[PLY as usize]};
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

/// O(n^2) algorithm to incrementally sort the move list as needed.
fn get_next_move(moves: &mut MoveList, move_scores: &mut MoveList, start_idx: &mut usize) -> Option<(u16, u16)> {
    let m_idx: usize = *start_idx;

    // no moves left
    if m_idx == move_scores.len {return None}

    // go through remaining moves
    let mut best_idx: usize = m_idx;
    let mut best_score: u16 = 0;
    for i in m_idx..move_scores.len {
        let score: u16 = move_scores.list[i];
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }

    // best move
    let m: u16 = moves.list[best_idx];

    // swap first remaining move with best move found and increment starting point
    move_scores.swap_unchecked(best_idx, m_idx);
    moves.swap_unchecked(best_idx, m_idx);
    *start_idx += 1;

    Some((m, best_score))
}

unsafe fn pvs(pv: bool, mut alpha: i16, mut beta: i16, mut depth: i8, in_check: bool, allow_null: bool) -> i16 {
    // search aborting
    if STOP { return 0 }
    if NODES & 2047 == 0 && START_TIME.unwrap().elapsed().as_millis() >= TIME {
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
    if depth <= 0 || PLY == MAX_PLY { return quiesce(alpha, beta) }

    // count the node
    NODES += 1;

    // probing hash table
    let mut hash_move: u16 = 0;
    let mut write_to_hash: bool = true;
    if let Some(res) = tt_probe(POS.state.zobrist) {
        // write to hash only if this search is of greater depth than the hash entry
        write_to_hash = depth > res.depth;

        // hash move for move ordering (immediate cutoff ~60% of the time there is one)
        hash_move = res.best_move;

        // hash score pruning
        // not at root, with shallower hash entries or near 50 move draws
        if PLY > 0 && POS.state.halfmove_clock <= 90 && res.depth >= depth &&
            match res.bound {
                Bound::EXACT => { !pv }, // want nice pv lines
                Bound::LOWER => { res.score >= beta },
                Bound::UPPER => { res.score <= alpha },
                _ => false
            } { return res.score }
    }

    // pruning
    if !pv && !in_check && beta.abs() < MATE_THRESHOLD {
        // pst only eval of position
        let lazy_eval: i16 = lazy_eval();

        // reverse futility pruning
        if depth <= 8 && lazy_eval >= beta + 120 * depth as i16 { return lazy_eval - 120 * depth as i16 }

        // null move pruning
        if allow_null && depth >= 3 && POS.state.phase >= 6 && lazy_eval >= beta {
            let ctx: (u16, u64) = do_null();
            let score: i16 = -pvs(false, -beta, -beta + 1, depth - 3, false, false);
            undo_null(ctx);
            if score >= beta {return score}
        }
    }

    // generating and scoring moves
    let mut m_idx: usize = 0;
    let mut moves = MoveList::default();
    let mut move_scores = MoveList::default();
    gen_moves::<ALL>(&mut moves);
    score_moves(&moves, &mut move_scores, hash_move, m_idx);

    // if no cutoff or alpha improvements are achieved then score is an upper bound
    let mut bound: u8 = Bound::UPPER;

    // is the threshold for late move reductions satisfied?
    let can_lmr: bool = depth >= 2 && PLY > 0 && !in_check;

    // going through moves
    PLY += 1;
    let mut best_move: u16 = 0;
    let mut best_score: i16 = -MAX;
    let mut count: u16 = 0;
    while let Some((m, m_score)) = get_next_move(&mut moves, &mut move_scores, &mut m_idx) {
        // make move and skip if not legal
        if do_move(m) { continue }
        count += 1;

        let gives_check: bool = is_in_check();

        // late move reductions
        let r: i8 = (can_lmr && !gives_check && count > 1 && m_score < 300) as i8;

        // score move
        let score: i16 = if count == 1 {
            -pvs(pv, -beta, -alpha, depth - 1, gives_check, false)
        } else {
            let zw_score: i16 = -pvs(false, -alpha - 1, -alpha, depth - 1 - r, gives_check, true);
            if (alpha != beta - 1 || r > 0) && zw_score > alpha {
                -pvs(pv, -beta, -alpha, depth - 1, gives_check, false)
            } else { zw_score }
        };

        undo_move();

        // alpha-beta pruning
        if score > best_score {
            best_score = score;
            best_move = m;

            // raise alpha
            if score > alpha {
                alpha = score;
                bound = Bound::EXACT;

                // beta prune
                if score >= beta {
                    // push to killer move table if not a capture
                    if m & 0b0100_0000_0000_0000 == 0 { kt_push(m) };

                    bound = Bound::LOWER;
                    break
                }
            }
        }
    }
    PLY -= 1;

    // set best move at root
    if PLY == 0 { BEST_MOVE = best_move }

    // check for (stale)mate
    if count == 0 { return (in_check as i16) * (-MAX + PLY as i16) }

    // write to hash if appropriate
    if write_to_hash { tt_push(POS.state.zobrist, best_move, depth, bound, best_score) }

    // fail-soft
    best_score
}

unsafe fn quiesce(mut alpha: i16, beta: i16) -> i16 {
    // count all quiescent nodes
    NODES += 1;

    // static eval as an initial guess
    let mut stand_pat: i16 = lazy_eval();

    // alpha-beta pruning
    if stand_pat >= beta { return stand_pat }
    if alpha < stand_pat { alpha = stand_pat }

    // generate and score moves
    let mut m_idx: usize = 0;
    let mut captures = MoveList::default();
    let mut scores = MoveList::default();
    gen_moves::<CAPTURES>(&mut captures);
    score_captures(&captures, &mut scores, m_idx);

    // delta pruning margin
    let margin = stand_pat + 200;

    // go through moves
    while let Some((m, m_score)) = get_next_move(&mut captures, &mut scores, &mut m_idx) {
        // delta pruning
        if margin + m_score as i16 / 5 < alpha { break }

        // make move and skip if not legal
        if do_move(m) { continue }
        let score: i16 = -quiesce(-beta, -alpha);
        undo_move();

        // alpha-beta pruning
        if score > stand_pat {
            stand_pat = score;
            if score > alpha {
                alpha = score;
                if score >= beta { break }
            }
        }
    }

    // fail-soft
    stand_pat
}

pub fn go() {
    unsafe {
    // initialise values
    NODES = 0;
    STOP = false;
    let mut best_move: u16 = 0;
    START_TIME = Some(Instant::now());

    // iterative deepening loop
    for d in 0..DEPTH {
        // determine if in check
        let in_check: bool = is_in_check();

        // get score
        let score: i16 = pvs(true, -MAX, MAX, d + 1, in_check, false);

        // end search if out of time
        let t: u128 = START_TIME.unwrap().elapsed().as_millis();
        if t >= TIME || STOP { break }

        // update best move
        best_move = BEST_MOVE;

        // uci output for the gui
        let (stype, sval): (&str, i16) = match score.abs() >= MATE_THRESHOLD {
            true => ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2),
            false => ("cp", score)
        };
        let nps: u32 = ((NODES as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = u16_to_uci(&best_move);
        println!("info depth {} score {} {} time {} nodes {} nps {} pv {}", d + 1, stype, sval, t, NODES, nps, pv_str);

        // stop searching if mate found
        if score.abs() >= MATE_THRESHOLD { break }
    }
    DEPTH = i8::MAX;
    TIME = u128::MAX;
    println!("bestmove {}", u16_to_uci(&best_move));
    kt_clear();
    }
}
