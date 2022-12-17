use super::{from, to, consts::*, position::*, tables::*, movegen::*, u16_to_uci};
use std::{cmp::{min, max}, time::Instant};

/// Determines what is done in the node:
/// 1. PV node - if node is on the current principle variation line,
/// no agressive pruning or reductions, etc
/// 2. In check - node should be extended and no pruning if in check
/// 3. Allow null - whether null move pruning should be allowed
struct NodeType(bool, bool, bool);

/// Search window for the node:
/// - Lower bound (alpha)
/// - Upper bound (beta)
struct Window(i16, i16);

/// Holds information needed for uci compatibility:
/// - start_time, allocated_time and abort_signal for ending search when time limit is reached
/// - node_count for outputting nodes and nps
/// - ply for correct checkmate scores
/// - best_move for sending to the gui
pub struct SearchContext {
    pub hash_table: HashTable,
    killer_table: KillerTable,
    pub allocated_time: u128,
    start_time: Instant,
    node_count: u64,
    best_move: u16,
    ply: i16,
    abort_signal: bool,
}

impl SearchContext{
    /// Constructs a new instance, given hash and killer tables.
    pub fn new(hash_table: HashTable, killer_table: KillerTable) -> Self {
        Self { hash_table, killer_table, start_time: Instant::now(), allocated_time: 1000, node_count: 0, best_move: 0, ply: 0, abort_signal: false }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.node_count = 0;
        self.best_move = 0;
        self.ply = 0;
        self.abort_signal = false;
    }
}

impl Position {
    /// Piece-square table eval of the position.
    #[inline(always)]
    fn lazy_eval(&self) -> i16 {
        let phase: i32 = std::cmp::min(self.state.phase as i32, TPHASE);
        SIDE_FACTOR[self.side_to_move] * ((phase * self.state.mg as i32 + (TPHASE - phase) * self.state.eg as i32) / TPHASE) as i16
    }

    /// Scores a capture based first on the value of the victim of the capture,
    /// then on the piece capturing.
    fn mvv_lva(&self, m: u16) -> u16 {
        let moved_pc: usize = self.squares[from!(m)] as usize;
        let captured_pc: usize = self.squares[to!(m)] as usize;
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

    /// Scores an arbitrary list of moves.
    fn score_moves(&self, moves: &MoveList, move_scores: &mut MoveList, hash_move: u16, ply: i16, kt: &KillerTable) {
        let killers: [u16; 3] = kt.0[ply as usize];
        for i in 0..moves.len { move_scores.push(self.score_move(moves.list[i], hash_move, killers)) }
    }

    /// Scores a list of moves, given they are all captures.
    fn score_captures(&self, moves: &MoveList, move_scores: &mut MoveList) {
        for i in 0..moves.len { move_scores.push(self.mvv_lva(moves.list[i])) }
    }
}

/// O(n^2) algorithm to incrementally sort the move list as needed.
fn pick_move(moves: &mut MoveList, move_scores: &mut MoveList, start_idx: &mut usize) -> Option<(u16, u16)> {
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
fn search(pos: &mut Position, nt: NodeType, w: Window, mut depth: i8, ctx: &mut SearchContext) -> i16 {
    // search aborting
    if ctx.abort_signal { return 0 }
    if ctx.node_count & 2047 == 0 && ctx.start_time.elapsed().as_millis() >= ctx.allocated_time {
        ctx.abort_signal = true;
        return 0
    }

    // draw detection - ignoring draws by 50 move rule as it caused engine to
    // make suboptimal moves if it thinks it is winning, in an effort to avoid a draw
    if pos.is_draw_by_repetition(2 + (ctx.ply == 0) as u8) || pos.is_draw_by_material() { return 0 }

    // extract node info
    let Window(mut alpha, mut beta): Window = w;
    let NodeType(pv, in_check, allow_null): NodeType = nt;

    // mate distance pruning
    alpha = max(alpha, -MAX + ctx.ply as i16);
    beta = min(beta, MAX - ctx.ply as i16 - 1);
    if alpha >= beta { return alpha }

    // check extensions
    depth += in_check as i8;

    // qsearch at depth 0
    if depth <= 0 { return qsearch(pos, Window(alpha, beta), &mut ctx.node_count) }

    // count the node
    ctx.node_count += 1;

    // probing hash table
    let mut hash_move: u16 = 0;
    let mut write_to_hash: bool = true;
    if let Some(res) = ctx.hash_table.probe(pos.state.zobrist, ctx.ply) {
        // write to hash only if this search is of greater depth than the hash entry
        write_to_hash = depth > res.depth;

        // hash move for move ordering
        hash_move = res.best_move;

        // hash score pruning
        // not at root, with shallower hash entries or near 50 move draws
        if ctx.ply > 0 && pos.state.halfmove_clock <= 90 && res.depth >= depth &&
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
            let copy: (u16, u64) = pos.do_null();
            let score: i16 = -search(pos, NodeType(false, false, false), Window(-beta, -beta + 1), depth - 3, ctx);
            pos.undo_null(copy);
            if score >= beta {return score}
        }
    }

    // generating and scoring moves
    let mut moves: MoveList = Default::default();
    let mut scores: MoveList = Default::default();
    pos.gen_moves::<ALL>(&mut moves);
    pos.score_moves(&moves, &mut scores, hash_move, ctx.ply, &ctx.killer_table);

    // if no cutoff or alpha improvements are achieved then score is an upper bound
    let mut bound: u8 = Bound::UPPER;

    // is the threshold for late move reductions satisfied?
    let can_lmr: bool = depth >= 2 && ctx.ply > 0 && !in_check;

    // going through moves
    ctx.ply += 1;
    let mut m_idx: usize = 0;
    let mut best_move: u16 = 0;
    let mut best_score: i16 = -MAX;
    let mut legal_moves: u16 = 0;
    while let Some((m, m_score)) = pick_move(&mut moves, &mut scores, &mut m_idx) {
        // make move and skip if not legal
        if pos.do_move(m) { continue }
        legal_moves += 1;

        // does the move give check?
        let gives_check: bool = pos.is_in_check();

        // late move reductions
        let r: i8 = (can_lmr && !gives_check && legal_moves > 1 && m_score < 300) as i8;

        // score move via principle variation search
        let score: i16 = if legal_moves == 1 {
            // first move is searched w/ a full window and no reductions
            -search(pos, NodeType(pv, gives_check, false), Window(-beta, -alpha), depth - 1, ctx)
        } else {
            // following moves are assumed to be worse and searched with a null window and all reductions/pruning
            let zw_score: i16 = -search(pos, NodeType(false, gives_check, true), Window(-alpha - 1, -alpha), depth - 1 - r, ctx);
            if (alpha != beta - 1 || r > 0) && zw_score > alpha {
                // if they are, in fact, not worse then a re-search with a full window and no reductions/pruning
                -search(pos, NodeType(pv, gives_check, false), Window(-beta, -alpha), depth - 1, ctx)
            } else { zw_score }
        };

        pos.undo_move();

        // alpha-beta pruning
        if score > best_score {
            best_score = score;
            best_move = m;
            if score > alpha {
                alpha = score;
                bound = Bound::EXACT;
                if score >= beta {
                    bound = Bound::LOWER;
                    // push to killer move table if not a capture
                    if m & 0b0100_0000_0000_0000 == 0 { ctx.killer_table.push(m, ctx.ply) };
                    break
                }
            }
        }
    }
    ctx.ply -= 1;

    // set best move at root
    if ctx.ply == 0 { ctx.best_move = best_move }

    // check for (stale)mate
    if legal_moves == 0 { return (in_check as i16) * (-MAX + ctx.ply as i16) }

    // write to hash if appropriate
    if write_to_hash { ctx.hash_table.push(pos.state.zobrist, best_move, depth, bound, best_score, ctx.ply) }

    // fail-soft
    best_score
}

/// Quiescence search:
/// - Fail-soft
/// - Searches capture sequences to reduce horizon effect
/// - Delta pruning
fn qsearch(pos: &mut Position, window: Window, node_count: &mut u64) -> i16 {
    // count all quiescent nodes
    *node_count += 1;

    // extract node info
    let Window(mut alpha, beta): Window = window;

    // static eval as an initial guess
    let mut stand_pat: i16 = pos.lazy_eval();

    // alpha-beta pruning
    if stand_pat >= beta { return stand_pat }
    if alpha < stand_pat { alpha = stand_pat }

    // generate and score moves
    let mut captures: MoveList = Default::default();
    let mut scores: MoveList = Default::default();
    pos.gen_moves::<CAPTURES>(&mut captures);
    pos.score_captures(&captures, &mut scores);

    // delta pruning margin
    let margin: i16 = stand_pat + 200;

    // go through moves
    let mut m_idx: usize = 0;
    while let Some((m, m_score)) = pick_move(&mut captures, &mut scores, &mut m_idx) {
        // delta pruning
        if margin + m_score as i16 / 5 < alpha { break }

        // make move and skip if not legal
        if pos.do_move(m) { continue }
        let score: i16 = -qsearch(pos, Window(-beta, -alpha), node_count);
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
    stand_pat
}

/// Root search function:
/// - Iterative deepening
/// - Handles uci output
pub fn go(pos: &mut Position, allocated_depth: i8, ctx: &mut SearchContext) {
    let mut best_move: u16 = 0;
    ctx.reset();

    // iterative deepening loop
    for d in 1..=allocated_depth {
        // determine if in check
        let in_check: bool = pos.is_in_check();

        // get score
        let score: i16 = search(pos, NodeType(true, in_check, false), Window(-MAX, MAX), d, ctx);

        // end search if out of time
        let t: u128 = ctx.start_time.elapsed().as_millis();
        if t >= ctx.allocated_time || ctx.abort_signal { break }

        // update best move
        best_move = ctx.best_move;

        // uci output for the gui
        let (stype, sval): (&str, i16) = match score.abs() >= MATE_THRESHOLD {
            true => ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2),
            false => ("cp", score)
        };
        let nps: u32 = ((ctx.node_count as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = u16_to_uci(&best_move);
        println!("info depth {} score {} {} time {} nodes {} nps {} pv {}", d, stype, sval, t, ctx.node_count, nps, pv_str);

        // stop searching if mate found
        if score.abs() >= MATE_THRESHOLD { break }
    }
    println!("bestmove {}", u16_to_uci(&best_move));
    ctx.killer_table.clear();
}
