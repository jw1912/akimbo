use super::{consts::*, position::Position, tables::{HashTable, KillerTable}, movegen::MoveList,u16_to_uci};
use std::{cmp::{min, max}, time::Instant};

/// Determines what is done in the node
struct NodeType(u8);
impl NodeType {
    fn encode(pv: bool, check: bool, null: bool) -> Self {
        Self(4 * u8::from(pv) + 2 * u8::from(check) + u8::from(null))
    }
}

/// Contains everything needed for a search.
pub struct SearchContext {
    pub hash_table: HashTable,
    killer_table: KillerTable,
    pub alloc_time: u128,
    time: Instant,
    nodes: u64,
    best_move: u16,
    ply: i16,
    abort: bool,
}

impl SearchContext{
    /// Constructs a new instance, given hash and killer tables.
    pub fn new(hash_table: HashTable, killer_table: KillerTable) -> Self {
        Self { hash_table, killer_table, time: Instant::now(), alloc_time: 1000, nodes: 0, best_move: 0, ply: 0, abort: false }
    }

    fn reset(&mut self) {
        self.time = Instant::now();
        self.nodes = 0;
        self.best_move = 0;
        self.ply = 0;
        self.abort = false;
    }
}

impl Position {
    /// Piece-square table eval of the position.
    #[inline]
    fn lazy_eval(&self) -> i16 {
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * self.state.mg as i32 + (TPHASE - phase) * self.state.eg as i32) / TPHASE) as i16
    }

    /// Scores a move.
    fn score_move(&self, m: u16, hash_move: u16, killers: &[u16; 3]) -> u16 {
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
        for i in 0..moves.len { move_scores.push(self.score_move(moves.list[i], hash_move, &killers)) }
    }

    /// Scores a list of moves, given they are all captures.
    fn score_captures(&self, moves: &MoveList, move_scores: &mut MoveList) {
        for i in 0..moves.len { move_scores.push(self.mvv_lva(moves.list[i])) }
    }
}

/// O(n^2) algorithm to incrementally sort the move list as needed.
fn pick_move(moves: &mut MoveList, scores: &mut MoveList) -> Option<(u16, u16)> {
    if scores.len == 0 {return None}
    let mut idx: usize = 0;
    let mut best: u16 = 0;
    let mut score: u16;
    for i in 0..scores.len {
        score = scores.list[i];
        if score > best {
            best = score;
            idx = i;
        }
    }
    scores.len -= 1;
    scores.list.swap(idx, scores.len);
    moves.list.swap(idx, scores.len);
    Some((moves.list[scores.len], best))
}

/// Main search function:
/// - Fail-soft negamax (alpha-beta pruning) framework
/// - Principle variation search
fn search(pos: &mut Position, nt: NodeType, mut alpha: i16, mut beta: i16, mut depth: i8, ctx: &mut SearchContext) -> i16 {
    // search aborting
    if ctx.abort { return 0 }
    if ctx.nodes & 2047 == 0 && ctx.time.elapsed().as_millis() >= ctx.alloc_time {
        ctx.abort = true;
        return 0
    }

    // draw detection - ignoring draws by 50 move rule as it caused engine to
    // make suboptimal moves if it thinks it is winning, in an effort to avoid a draw
    if pos.is_draw_by_repetition(2 + u8::from(ctx.ply == 0)) || pos.is_draw_by_material() { return 0 }

    // extract node info
    let (pv, in_check, allow_null): (bool, bool, bool) = (nt.0 & 4 > 0, nt.0 & 2 > 0, nt.0 & 1 > 0);

    // mate distance pruning
    alpha = max(alpha, -MAX + ctx.ply);
    beta = min(beta, MAX - ctx.ply - 1);
    if alpha >= beta { return alpha }

    // check extensions
    depth += i8::from(in_check);

    // qsearch at depth 0
    if depth <= 0 { return qsearch(pos, alpha, beta, &mut ctx.nodes) }

    // count the node
    ctx.nodes += 1;

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
        let margin: i16 = lazy_eval - 120 * i16::from(depth);
        if depth <= 8 && margin >= beta { return margin }

        // null move pruning
        if allow_null && depth >= 3 && pos.phase >= 6 && lazy_eval >= beta {
            let copy: (u16, u64) = pos.do_null();
            let score: i16 = -search(pos, NodeType::encode(false, false, false), -beta, -beta + 1, depth - 3, ctx);
            pos.undo_null(copy);
            if score >= beta {return score}
        }
    }

    // generating and scoring moves
    let mut moves: MoveList = MoveList::default();
    let mut scores: MoveList = MoveList::default();
    pos.gen_moves::<ALL>(&mut moves);
    pos.score_moves(&moves, &mut scores, hash_move, ctx.ply, &ctx.killer_table);

    // if no cutoff or alpha improvements are achieved then score is an upper bound
    let mut bound: u8 = Bound::UPPER;

    // is the threshold for late move reductions satisfied?
    let can_lmr: bool = depth >= 2 && ctx.ply > 0 && !in_check;

    // going through moves
    ctx.ply += 1;
    let mut best_move: u16 = 0;
    let mut best_score: i16 = -MAX;
    let mut legal_moves: u16 = 0;
    while let Some((m, m_score)) = pick_move(&mut moves, &mut scores) {
        // make move and skip if not legal
        if pos.do_move(m) { continue }
        legal_moves += 1;

        // does the move give check?
        let gives_check: bool = pos.is_in_check();

        // late move reductions
        let reduce: i8 = i8::from(can_lmr && !gives_check && legal_moves > 1 && m_score < 300);

        // score move via principle variation search
        let score: i16 = if legal_moves == 1 {
            // first move is searched with a full window and no reductions
            -search(pos, NodeType::encode(pv, gives_check, false), -beta, -alpha, depth - 1, ctx)
        } else {
            // following moves are assumed to be worse and searched with a null window and all reductions/pruning
            let zw_score: i16 = -search(pos, NodeType::encode(false, gives_check, true), -alpha - 1, -alpha, depth - 1 - reduce, ctx);
            if (alpha != beta - 1 || reduce > 0) && zw_score > alpha {
                // if they are, in fact, not worse then a re-search with a full window and no reductions/pruning
                -search(pos, NodeType::encode(pv, gives_check, false), -beta, -alpha, depth - 1, ctx)
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
    if legal_moves == 0 { return i16::from(in_check) * (-MAX + ctx.ply) }

    // write to hash if appropriate
    if write_to_hash { ctx.hash_table.push(pos.state.zobrist, best_move, depth, bound, best_score, ctx.ply) }

    // fail-soft
    best_score
}

/// Quiescence search:
/// - Fail-soft
/// - Searches capture sequences to reduce horizon effect
/// - Delta pruning
fn qsearch(pos: &mut Position, mut alpha: i16, beta: i16, node_count: &mut u64) -> i16 {
    // count all quiescent nodes
    *node_count += 1;

    // static eval as an initial guess
    let mut stand_pat: i16 = pos.lazy_eval();

    // alpha-beta pruning
    if stand_pat >= beta { return stand_pat }
    if alpha < stand_pat { alpha = stand_pat }

    // generate and score moves
    let mut captures: MoveList = MoveList::default();
    let mut scores: MoveList = MoveList::default();
    pos.gen_moves::<CAPTURES>(&mut captures);
    pos.score_captures(&captures, &mut scores);

    // go through moves
    while let Some((m, m_score)) = pick_move(&mut captures, &mut scores) {
        // delta pruning
        if stand_pat + m_score as i16 / 5 + 200 < alpha { break }

        // make move and skip if not legal
        if pos.do_move(m) { continue }
        let score: i16 = -qsearch(pos, -beta, -alpha, node_count);
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
        let score: i16 = search(pos, NodeType::encode(true, in_check, false), -MAX, MAX, d, ctx);

        // end search if out of time
        let t: u128 = ctx.time.elapsed().as_millis();
        if t >= ctx.alloc_time || ctx.abort { break }

        // update best move
        best_move = ctx.best_move;

        // uci output for the gui
        let (stype, sval): (&str, i16) = if score.abs() >= MATE_THRESHOLD {
            ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2)
        } else {
            ("cp", score)
        };
        let nps: u32 = ((ctx.nodes as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = u16_to_uci(best_move);
        println!("info depth {} score {} {} time {} nodes {} nps {} pv {}", d, stype, sval, t, ctx.nodes, nps, pv_str);

        // stop searching if mate found
        if score.abs() >= MATE_THRESHOLD { break }
    }
    println!("bestmove {}", u16_to_uci(best_move));
    ctx.killer_table.clear();
}
