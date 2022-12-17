use super::{from, to, consts::*, position::*, tables::*, movegen::*, u16_to_uci};
use std::{cmp::{min, max}, time::Instant};

/// Determines what is done in the node:
/// 1. PV node - if node is on the current principle variation line, no agressive pruniong, etc
/// 2. In check - node should be extended and no pruning if in check
/// 3. Allow null - whether null move pruning should be allowed
struct NodeType(bool, bool, bool);

/// Holds information needed for uci compatibility:
/// - start_time, allocated_time and abort_signal for ending search when time limit is reached
/// - node_count for outputting nodes and nps
/// - ply for correct checkmate scores
/// - best_move for sending to the gui
struct SearchStats {
    start_time: Instant,
    allocated_time: u128,
    node_count: u64,
    best_move: u16,
    ply: i8,
    abort_signal: bool,
}

impl Position {
    /// Piece-square table eval of the position
    #[inline(always)]
    fn lazy_eval(&self) -> i16 {
        let phase: i32 = std::cmp::min(self.state.phase as i32, TPHASE);
        SIDE_FACTOR[self.side_to_move] * ((phase * self.state.mg as i32 + (TPHASE - phase) * self.state.eg as i32) / TPHASE) as i16
    }

    /// Scores a capture based first on the value of the victim of the capture,
    /// then on the piece capturing
    fn mvv_lva(&self, m: u16) -> u16 {
        let from_idx: usize = from!(m);
        let to_idx: usize = to!(m);
        let moved_pc: usize = self.squares[from_idx] as usize;
        let captured_pc: usize = self.squares[to_idx] as usize;
        MVV_LVA[captured_pc][moved_pc]
    }

    /// Scores a move.
    /// 1. Hash move
    /// 2. Captures, sorted by MVV-LVA
    /// 3. Promotions
    /// 4. Killer moves
    /// 5. Quiet moves
    fn score_move(&self, m: u16, hash_move: u16, killers: [u16; 3]) -> u16 {
        if m == hash_move {
            HASH_MOVE
        } else if m & 0b0100_0000_0000_0000 > 0 {
            self.mvv_lva(m)
        } else if m & 0b1000_0000_0000_0000 > 0 {
            PROMOTION
        } else if killers.contains(&m) {
            KILLER
        } else {
            QUIET
        }
    }

    /// Scores an arbitrary list of moves
    fn score_moves(&self, moves: &MoveList, move_scores: &mut MoveList, hash_move: u16, ply: i8) {
        let killers: [u16; 3] = kt_get(ply);
        for i in 0..moves.len { move_scores.push(self.score_move(moves.list[i], hash_move, killers)) }
    }

    /// Scores a list of moves, given they are all captures
    fn score_captures(&self, moves: &MoveList, move_scores: &mut MoveList) {
        for i in 0..moves.len { move_scores.push(self.mvv_lva(moves.list[i])) }
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
    move_scores.list.swap(best_idx, m_idx);
    moves.list.swap(best_idx, m_idx);
    *start_idx += 1;

    Some((m, best_score))
}

/// Main search function:
/// - Fail-soft negamax (alpha-beta pruning) framework
/// - Principle variation search
fn search(pos: &mut Position, node_type: NodeType, stats: &mut SearchStats, mut alpha: i16, mut beta: i16, mut depth: i8) -> i16 {
    // search aborting
    if stats.abort_signal { return 0 }
    if stats.node_count & 2047 == 0 && stats.start_time.elapsed().as_millis() >= stats.allocated_time {
        stats.abort_signal = true;
        return 0
    }

    // draw detection - ignoring draws by 50 move rule as it caused engine to
    // make suboptimal moves if it thinks it is winning
    if pos.is_draw_by_repetition(2 + (stats.ply == 0) as u8) || pos.is_draw_by_material() { return 0 }

    // mate distance pruning
    alpha = max(alpha, -MAX + stats.ply as i16);
    beta = min(beta, MAX - stats.ply as i16 - 1);
    if alpha >= beta { return alpha }

    let NodeType(pv, in_check, allow_null): NodeType = node_type;

    // check extensions
    depth += in_check as i8;

    // qsearch at depth 0
    if depth <= 0 || stats.ply == MAX_PLY { return quiesce(pos, &mut stats.node_count, alpha, beta) }

    // count the node
    stats.node_count += 1;

    // probing hash table
    let mut hash_move: u16 = 0;
    let mut write_to_hash: bool = true;
    if let Some(res) = tt_probe(pos.state.zobrist, stats.ply) {
        // write to hash only if this search is of greater depth than the hash entry
        write_to_hash = depth > res.depth;

        // hash move for move ordering (immediate cutoff ~60% of the time there is one)
        hash_move = res.best_move;

        // hash score pruning
        // not at root, with shallower hash entries or near 50 move draws
        if stats.ply > 0 && pos.state.halfmove_clock <= 90 && res.depth >= depth &&
            match res.bound {
                Bound::EXACT => !pv, // want nice pv lines
                Bound::LOWER => res.score >= beta,
                Bound::UPPER => res.score <= alpha,
                _ => false
            } { return res.score }
    }

    // pruning
    if !pv && !in_check && beta.abs() < MATE_THRESHOLD {
        // pst only eval of position
        let lazy_eval: i16 = pos.lazy_eval();

        // reverse futility pruning
        if depth <= 8 && lazy_eval >= beta + 120 * depth as i16 { return lazy_eval - 120 * depth as i16 }

        // null move pruning
        if allow_null && depth >= 3 && pos.state.phase >= 6 && lazy_eval >= beta {
            let ctx: (u16, u64) = pos.do_null();
            let score: i16 = -search(pos, NodeType(false, false, false), stats, -beta, -beta + 1, depth - 3);
            pos.undo_null(ctx);
            if score >= beta {return score}
        }
    }

    // generating and scoring moves
    let mut moves = MoveList::default();
    let mut move_scores = MoveList::default();
    pos.gen_moves::<ALL>(&mut moves);
    pos.score_moves(&moves, &mut move_scores, hash_move, stats.ply);

    // if no cutoff or alpha improvements are achieved then score is an upper bound
    let mut bound: u8 = Bound::UPPER;

    // is the threshold for late move reductions satisfied?
    let can_lmr: bool = depth >= 2 && stats.ply > 0 && !in_check;

    // going through moves
    stats.ply += 1;
    let mut m_idx: usize = 0;
    let mut best_move: u16 = 0;
    let mut best_score: i16 = -MAX;
    let mut count: u16 = 0;
    while let Some((m, m_score)) = get_next_move(&mut moves, &mut move_scores, &mut m_idx) {
        // make move and skip if not legal
        if pos.do_move(m) { continue }
        count += 1;

        // does the move give check?
        let gives_check: bool = pos.is_in_check();

        // late move reductions
        let r: i8 = (can_lmr && !gives_check && count > 1 && m_score < 300) as i8;

        // score move
        let score: i16 = if count == 1 {
            -search(pos, NodeType(pv, gives_check, false), stats, -beta, -alpha, depth - 1)
        } else {
            let zw_score: i16 = -search(pos, NodeType(false, gives_check, true), stats, -alpha - 1, -alpha, depth - 1 - r);
            if (alpha != beta - 1 || r > 0) && zw_score > alpha {
                -search(pos, NodeType(pv, gives_check, false), stats, -beta, -alpha, depth - 1)
            } else { zw_score }
        };

        pos.undo_move();

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
                    if m & 0b0100_0000_0000_0000 == 0 { kt_push(m, stats.ply) };

                    bound = Bound::LOWER;
                    break
                }
            }
        }
    }
    stats.ply -= 1;

    // set best move at root
    if stats.ply == 0 { stats.best_move = best_move }

    // check for (stale)mate
    if count == 0 { return (in_check as i16) * (-MAX + stats.ply as i16) }

    // write to hash if appropriate
    if write_to_hash { tt_push(pos.state.zobrist, best_move, depth, bound, best_score, stats.ply) }

    // fail-soft
    best_score
}

/// Quiescence search:
/// - Fail-soft
/// - Searches capture sequences to reduce horizon effect
/// - Delta pruning
fn quiesce(pos: &mut Position, node_count: &mut u64, mut alpha: i16, beta: i16) -> i16 {
    // count all quiescent nodes
    *node_count += 1;

    // static eval as an initial guess
    let mut stand_pat: i16 = pos.lazy_eval();

    // alpha-beta pruning
    if stand_pat >= beta { return stand_pat }
    if alpha < stand_pat { alpha = stand_pat }

    // generate and score moves
    let mut captures = MoveList::default();
    let mut scores = MoveList::default();
    pos.gen_moves::<CAPTURES>(&mut captures);
    pos.score_captures(&captures, &mut scores);

    // delta pruning margin
    let margin: i16 = stand_pat + 200;

    // go through moves
    let mut m_idx: usize = 0;
    while let Some((m, m_score)) = get_next_move(&mut captures, &mut scores, &mut m_idx) {
        // delta pruning
        if margin + m_score as i16 / 5 < alpha { break }

        // make move and skip if not legal
        if pos.do_move(m) { continue }
        let score: i16 = -quiesce(pos, node_count, -beta, -alpha);
        pos.undo_move();

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

/// Root search function:
/// - Iterative deepening
/// - Handles uci output
pub fn go(pos: &mut Position, allocated_time: u128) {
    // initialise values
    let mut stats: SearchStats = SearchStats { start_time: Instant::now(), node_count: 0, best_move: 0, ply: 0, abort_signal: false, allocated_time };
    let mut best_move: u16 = 0;

    // iterative deepening loop
    for d in 0..MAX_PLY {
        // determine if in check
        let in_check: bool = pos.is_in_check();

        // get score
        let score: i16 = search(pos, NodeType(true, in_check, false), &mut stats, -MAX, MAX, d + 1);

        // end search if out of time
        let t: u128 = stats.start_time.elapsed().as_millis();
        if t >= stats.allocated_time || stats.abort_signal { break }

        // update best move
        best_move = stats.best_move;

        // uci output for the gui
        let (stype, sval): (&str, i16) = match score.abs() >= MATE_THRESHOLD {
            true => ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2),
            false => ("cp", score)
        };
        let nps: u32 = ((stats.node_count as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = u16_to_uci(&best_move);
        println!("info depth {} score {} {} time {} nodes {} nps {} pv {}", d + 1, stype, sval, t, stats.node_count, nps, pv_str);

        // stop searching if mate found
        if score.abs() >= MATE_THRESHOLD { break }
    }
    println!("bestmove {}", u16_to_uci(&best_move));
    kt_clear();
}
