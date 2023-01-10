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
    ply: i16,
    abort: bool,
}

impl SearchContext{
    pub fn new(hash_table: HashTable, killer_table: KillerTable) -> Self {
        Self { hash_table, killer_table, time: Instant::now(), alloc_time: 1000, nodes: 0, ply: 0, abort: false }
    }

    fn reset(&mut self) {
        self.time = Instant::now();
        self.nodes = 0;
        self.ply = 0;
        self.abort = false;
    }
}

impl Position {
    fn score_move(&self, m: u16, hash_move: u16, killers: &[u16; KILLERS_PER_PLY]) -> u16 {
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

    fn score(&self, moves: &MoveList, hash_move: u16, ply: i16, kt: &KillerTable) -> MoveList {
        let mut scores: MoveList = MoveList::default();
        let killers: [u16; KILLERS_PER_PLY] = kt.0[ply as usize];
        for i in 0..moves.len { scores.push(self.score_move(moves.list[i], hash_move, &killers)) }
        scores
    }

    fn score_captures(&self, moves: &MoveList) -> MoveList {
        let mut scores: MoveList = MoveList::default();
        for i in 0..moves.len { scores.push(self.mvv_lva(moves.list[i])) }
        scores
    }
}

impl MoveList {
    // O(n^2) algorithm to incrementally sort the move list as needed.
    fn pick(&mut self, scores: &mut MoveList) -> Option<(u16, u16)> {
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
        self.list.swap(idx, scores.len);
        Some((self.list[scores.len], best))
    }
}

/// Main search function:
/// - Fail-soft negamax (alpha-beta pruning) framework
/// - Principle variation search
fn search(pos: &mut Position, nt: NodeType, mut alpha: i16, mut beta: i16, mut depth: i8, ctx: &mut SearchContext, pv_line: &mut Vec<u16>) -> i16 {
    // search aborting
    if ctx.abort { return 0 }
    if ctx.nodes & 2047 == 0 && ctx.time.elapsed().as_millis() >= ctx.alloc_time {
        ctx.abort = true;
        return 0
    }

    // draw detection
    if pos.fifty_draw() || pos.repetition_draw(2 + u8::from(ctx.ply == 0)) || pos.material_draw() { return 0 }

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
    ctx.nodes += 1;

    // probing hash table
    let mut hash_move: u16 = 0;
    let mut write_to_hash: bool = true;
    if let Some(res) = ctx.hash_table.probe(pos.state.zobrist, ctx.ply) {
        write_to_hash = depth > res.depth;
        hash_move = res.best_move;
        // hash score pruning
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
        let lazy_eval: i16 = pos.lazy_eval();

        // reverse futility pruning
        let margin: i16 = lazy_eval - 120 * i16::from(depth);
        if depth <= 8 && margin >= beta { return margin }

        // null move pruning
        if allow_null && depth >= 3 && pos.phase >= 6 && lazy_eval >= beta {
            let copy: (u16, u64) = pos.do_null();
            let score: i16 = -search(pos, NodeType::encode(false, false, false), -beta, -beta + 1, depth - 3, ctx, &mut Vec::new());
            pos.undo_null(copy);
            if score >= beta {return score}
        }
    }

    // generate and score moves
    let mut moves: MoveList = pos.gen_moves::<ALL>();
    let mut scores: MoveList = pos.score(&moves, hash_move, ctx.ply, &ctx.killer_table);

    // is the threshold for late move reductions satisfied?
    let can_lmr: bool = depth >= 2 && ctx.ply > 0 && !in_check;

    ctx.ply += 1;
    let mut bound: u8 = Bound::UPPER;
    let mut best_move: u16 = 0;
    let mut best_score: i16 = -MAX;
    let mut legal_moves: u16 = 0;
    while let Some((m, m_score)) = moves.pick(&mut scores) {
        if pos.do_move(m) { continue }
        legal_moves += 1;

        // late move reductions
        let gives_check: bool = pos.is_in_check();
        let reduce: i8 = i8::from(can_lmr && !gives_check && legal_moves > 1 && m_score < 300);

        // pvs
        let mut sub_pv: Vec<u16> = Vec::new();
        let score: i16 = if legal_moves == 1 {
            -search(pos, NodeType::encode(pv, gives_check, false), -beta, -alpha, depth - 1, ctx, &mut sub_pv)
        } else {
            let zw_score: i16 = -search(pos, NodeType::encode(false, gives_check, true), -alpha - 1, -alpha, depth - 1 - reduce, ctx, &mut sub_pv);
            if (alpha != beta - 1 || reduce > 0) && zw_score > alpha {
                -search(pos, NodeType::encode(pv, gives_check, false), -beta, -alpha, depth - 1, ctx, &mut sub_pv)
            } else { zw_score }
        };

        pos.undo_move();

        if score > best_score {
            best_score = score;
            best_move = m;
            if score > alpha {
                alpha = score;
                bound = Bound::EXACT;
                // update pv
                pv_line.clear();
                pv_line.push(m);
                pv_line.append(&mut sub_pv);
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
    if legal_moves == 0 { return i16::from(in_check) * (-MAX + ctx.ply) }
    if write_to_hash && !ctx.abort { ctx.hash_table.push(pos.state.zobrist, best_move, depth, bound, best_score, ctx.ply) }

    best_score
}

/// Quiescence search:
/// - Fail-soft
/// - Delta pruning
fn qsearch(pos: &mut Position, mut alpha: i16, beta: i16, nodes: &mut u64) -> i16 {
    *nodes += 1;
    let mut stand_pat: i16 = pos.eval();

    if stand_pat >= beta { return stand_pat }
    if alpha < stand_pat { alpha = stand_pat }

    // generate and score moves
    let mut captures: MoveList = pos.gen_moves::<CAPTURES>();
    let mut scores: MoveList = pos.score_captures(&captures);

    while let Some((m, m_score)) = captures.pick(&mut scores) {
        // delta pruning
        if stand_pat + m_score as i16 / 5 + DELTA_MARGIN < alpha { break }

        if pos.do_move(m) { continue }
        let score: i16 = -qsearch(pos, -beta, -alpha, nodes);
        pos.undo_move();

        if score > stand_pat {
            stand_pat = score;
            if score > alpha {
                alpha = score;
                if score >= beta { return score }
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

    for d in 1..=allocated_depth {
        let in_check: bool = pos.is_in_check();
        let mut pv_line: Vec<u16> = Vec::new();
        let score: i16 = search(pos, NodeType::encode(true, in_check, false), -MAX, MAX, d, ctx, &mut pv_line);

        // end search if out of time
        let t: u128 = ctx.time.elapsed().as_millis();
        if t >= ctx.alloc_time || ctx.abort { break }

        best_move = pv_line[0];
        let (stype, sval): (&str, i16) = if score.abs() >= MATE_THRESHOLD {
            ("mate", if score < 0 { score.abs() - MAX } else { MAX - score + 1 } / 2)
        } else {
            ("cp", score)
        };
        let nps: u32 = ((ctx.nodes as f64) * 1000.0 / (t as f64)) as u32;
        let pv_str: String = pv_line.iter().map(|m| u16_to_uci(pos, *m)).collect::<String>();
        println!("info depth {} score {} {} time {} nodes {} nps {} pv {}", d, stype, sval, t, ctx.nodes, nps, pv_str);

        // stop searching if mate found
        if score.abs() >= MATE_THRESHOLD { break }
    }
    println!("bestmove {}", u16_to_uci(pos, best_move));
    ctx.killer_table.clear();
}
