use std::{fmt::Write, time::Instant};

use super::{
    consts::{Bound, MoveScore, Piece, Score},
    moves::{Move, MoveList},
    position::Position,
    tables::NodeTable,
    thread::ThreadData,
};

fn mvv_lva(mov: Move, pos: &Position) -> i32 {
    8 * pos.get_pc(mov.bb_to()) as i32 - mov.moved_pc() as i32
}

pub fn go(
    start: &Position,
    eng: &mut ThreadData,
    main_thread: bool,
    max_depth: i32,
    soft_bound: f64,
    soft_nodes: u64,
) -> (Move, i32) {
    // reset engine
    eng.ntable = NodeTable::default();
    eng.plied.clear();
    eng.timing = Instant::now();
    eng.nodes = 0;
    eng.qnodes = 0;
    eng.ply = 0;
    eng.best_move = Move::NULL;
    eng.seldepth = 0;

    let mut best_move = Move::NULL;
    let mut pos = *start;
    let mut eval = 0;
    let mut score = 0;
    pos.check = pos.in_check();

    // iterative deepening loop
    for d in 1..=max_depth {
        eval = if d < 7 {
            pvs(&pos, eng, -Score::MAX, Score::MAX, d, false)
        } else {
            aspiration(&pos, eng, eval, d, &mut best_move)
        };

        if eng.stop_is_set() {
            break;
        }

        best_move = eng.best_move;
        score = eval;

        let nodes = eng.nodes + eng.qnodes;

        // UCI output
        if main_thread {
            let score = if eval.abs() >= Score::MATE {
                format!(
                    "score mate {}",
                    if eval < 0 {
                        eval.abs() - Score::MAX
                    } else {
                        Score::MAX - eval + 1
                    } / 2
                )
            } else {
                format!("score cp {eval}")
            };
            let t = eng.timing.elapsed().as_millis();
            let nps = (1000.0 * nodes as f64 / t as f64) as u32;
            let pv_line = &eng.plied[0].pv_line;
            let pv = pv_line.iter().fold(String::new(), |mut pv_str, mov| {
                write!(&mut pv_str, "{} ", mov.to_uci()).unwrap();
                pv_str
            });
            println!(
                "info depth {d} seldepth {} {score} time {t} nodes {nodes} nps {nps:.0} pv {pv}",
                eng.seldepth
            );

            let frac = eng.ntable.get(best_move) as f64 / nodes as f64;
            if eng.timing.elapsed().as_millis() as f64
                >= soft_bound * if d > 8 { (1.5 - frac) * 1.35 } else { 1.0 }
            {
                break;
            }
        }

        if nodes > soft_nodes {
            break;
        }
    }

    (best_move, score)
}

fn aspiration(
    pos: &Position,
    eng: &mut ThreadData,
    mut score: i32,
    max_depth: i32,
    best_move: &mut Move,
) -> i32 {
    let mut delta = 25;
    let mut alpha = (-Score::MAX).max(score - delta);
    let mut beta = Score::MAX.min(score + delta);
    let mut depth = max_depth;

    loop {
        score = pvs(pos, eng, alpha, beta, depth, false);

        if eng.stop_is_set() {
            return 0;
        }

        if score <= alpha {
            beta = (alpha + beta) / 2;
            alpha = (-Score::MAX).max(alpha - delta);
            depth = max_depth;
        } else if score >= beta {
            beta = Score::MAX.min(beta + delta);
            *best_move = eng.best_move;
            depth -= 1;
        } else {
            return score;
        }

        delta *= 2;
    }
}

fn qs(pos: &Position, eng: &mut ThreadData, mut alpha: i32, beta: i32) -> i32 {
    eng.seldepth = eng.seldepth.max(eng.ply);

    let mut eval = pos.eval();

    // stand-pat
    if eval >= beta {
        return eval;
    }

    alpha = alpha.max(eval);

    let hash = pos.hash();

    // probe hash table for cutoff
    if let Some(entry) = eng.tt.probe(hash, eng.ply) {
        let tt_score = entry.score();
        if match entry.bound() {
            Bound::LOWER => tt_score >= beta,
            Bound::UPPER => tt_score <= alpha,
            _ => true,
        } {
            return tt_score;
        }
    }

    let mut caps = pos.movegen::<false>();
    let mut scores = [0; 252];

    caps.iter()
        .enumerate()
        .for_each(|(i, &cap)| scores[i] = mvv_lva(cap, pos));

    let mut best_move = Move::NULL;
    let mut bound = Bound::UPPER;

    eng.ply += 1;

    while let Some((mov, _)) = caps.pick(&mut scores) {
        // static exchange eval pruning
        if !pos.see(mov, 1) {
            continue;
        }

        let mut new = *pos;
        if new.make(mov) {
            continue;
        }

        eng.qnodes += 1;

        let score = -qs(&new, eng, -beta, -alpha);

        if score <= eval {
            continue;
        }

        eval = score;
        best_move = mov;

        if eval >= beta {
            bound = Bound::LOWER;
            break;
        }

        alpha = alpha.max(eval);
    }

    eng.ply -= 1;

    eng.tt.push(hash, best_move, 0, bound, eval, eng.ply);

    eval
}

#[allow(clippy::too_many_arguments)]
fn pvs(
    pos: &Position,
    eng: &mut ThreadData,
    mut alpha: i32,
    mut beta: i32,
    mut depth: i32,
    null: bool,
) -> i32 {
    // stopping search
    if eng.stop_is_set() {
        return 0;
    }

    if eng.nodes & 1023 == 0
        && (eng.timing.elapsed().as_millis() >= eng.max_time
            || eng.nodes + eng.qnodes >= eng.max_nodes)
    {
        eng.store_stop(true);
        return 0;
    }

    let hash = pos.hash();

    // clear pv line
    eng.plied[eng.ply].pv_line.clear();

    if eng.ply > 0 {
        // draw detection
        if pos.draw() || eng.repetition(pos, hash, eng.ply == 0) {
            return Score::DRAW;
        }

        // mate distance pruning
        alpha = alpha.max(eng.ply - Score::MAX);
        beta = beta.min(Score::MAX - eng.ply - 1);
        if alpha >= beta {
            return alpha;
        }

        // check extensions - not on root
        depth += i32::from(pos.check);
    }

    // drop into quiescence search
    if depth <= 0 || eng.ply == 95 {
        return qs(pos, eng, alpha, beta);
    }

    let pv_node = beta > alpha + 1;
    let s_mov = eng.plied[eng.ply].singular;
    let singular = s_mov != Move::NULL;

    let mut eval = pos.eval();
    let mut tt_move = Move::NULL;
    let mut tt_score = -Score::MAX;
    let mut try_singular = !singular && depth >= 8 && eng.ply > 0;

    // probing hash table
    if let Some(entry) = eng.tt.probe(hash, eng.ply) {
        let bound = entry.bound();
        tt_move = entry.best_move(pos);
        tt_score = entry.score();
        try_singular &=
            entry.depth() >= depth - 3 && bound != Bound::UPPER && tt_score.abs() < Score::MATE;

        // tt cutoffs
        if !singular
            && !pv_node
            && depth <= entry.depth()
            && match bound {
                Bound::LOWER => tt_score >= beta,
                Bound::UPPER => tt_score <= alpha,
                _ => true,
            }
        {
            return tt_score;
        }

        // use tt score instead of static eval
        if !((eval > tt_score && bound == Bound::LOWER)
            || (eval < tt_score && bound == Bound::UPPER))
        {
            eval = tt_score;
        }
    }

    // improving heuristic
    eng.plied[eng.ply].eval = eval;
    let improving = eng.ply > 1 && eval > eng.plied[eng.ply - 2].eval;

    // pruning
    let mut can_prune = !pv_node && !pos.check;
    if can_prune && beta.abs() < Score::MATE {
        // reverse futility pruning
        if depth <= 8 && eval >= beta + 80 * depth / if improving { 2 } else { 1 } {
            return eval;
        }

        // razoring
        if depth <= 2 && eval + 400 * depth < alpha {
            let qeval = qs(pos, eng, alpha, beta);

            if qeval < alpha {
                return qeval;
            }
        }

        // null move pruning
        if null && depth >= 3 && pos.phase > 2 && eval >= beta {
            let mut new = *pos;
            let r = 3 + depth / 3;

            eng.push(hash);
            new.make_null();
            eng.plied[eng.ply].played = Move::NULL;

            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false);

            eng.pop();

            if nw >= Score::MATE {
                return beta;
            }

            if nw >= beta {
                return nw;
            }
        }
    }

    // internal iterative reduction
    if depth >= 4 && tt_move == Move::NULL {
        depth -= 1
    }

    // generating moves
    let mut moves = pos.movegen::<true>();

    let prev = eng.plied.prev_move(eng.ply, 1);
    let prevs = [prev, eng.plied.prev_move(eng.ply, 2)];

    let threats = pos.threats();
    let killer = eng.plied[eng.ply].killer;

    // scoring moves
    let mut scores = [0; 252];
    moves.iter().enumerate().for_each(|(i, &mov)| {
        scores[i] = if mov == tt_move {
            MoveScore::HASH
        } else if mov.is_en_passant() {
            MoveScore::CAPTURE + 16
        } else if mov.is_capture() {
            MoveScore::CAPTURE * i32::from(pos.see(mov, 0)) + mvv_lva(mov, pos)
        } else if mov.is_promo() {
            MoveScore::PROMO + i32::from(mov.flag() & 7)
        } else if mov == killer {
            MoveScore::KILLER
        } else {
            eng.htable.get_score(pos.stm(), mov, prevs, threats)
        }
    });

    let mut legal = 0;
    let mut bound = Bound::UPPER;
    let mut best_score = -Score::MAX;
    let mut best_move = tt_move;
    let mut quiets_tried = MoveList::ZEROED;

    let can_lmr = depth > 1 && eng.ply > 0 && !pos.check;
    let lmr_base = (depth as f64).ln() / 2.67;
    can_prune &= eng.mloop;

    eng.push(hash);
    eng.plied[eng.ply].dbl_exts = eng.plied[eng.ply - 1].dbl_exts;

    while let Some((mov, ms)) = moves.pick(&mut scores) {
        // move is singular in a singular search
        if mov == s_mov {
            continue;
        }

        // pre-move pruning
        if can_prune && best_score.abs() < Score::MATE {
            // late move pruning
            if ms < MoveScore::KILLER && legal > 2 + depth * depth / if improving { 1 } else { 2 } {
                break;
            }

            // static exchange eval pruning
            let margin = if mov.is_capture() { -90 } else { -50 };
            if depth < 7 && ms < MoveScore::CAPTURE && !pos.see(mov, margin * depth) {
                continue;
            }
        }

        // make move and skip if not legal
        let mut new = *pos;
        if new.make(mov) {
            continue;
        }

        new.check = new.in_check();
        eng.nodes += 1;
        legal += 1;

        if !mov.is_noisy() {
            quiets_tried.add(mov);
        }

        // singular extensions
        let ext = if try_singular && mov == tt_move {
            let s_beta = tt_score - depth * 2;

            eng.pop();
            eng.plied[eng.ply].singular = mov;

            let s_score = pvs(pos, eng, s_beta - 1, s_beta, (depth - 1) / 2, false);

            eng.plied[eng.ply].singular = Move::NULL;
            eng.push(hash);

            if s_score < s_beta {
                if !pv_node && s_score < s_beta - 25 && eng.plied[eng.ply].dbl_exts < 5 {
                    eng.plied[eng.ply].dbl_exts += 1;
                    2
                } else {
                    1
                }
            } else if tt_score >= beta {
                -1
            } else {
                0
            }
        } else {
            0
        };

        // reductions
        let reduce = if can_lmr && ms < MoveScore::KILLER {
            // late move reductions - Viridithas values used
            let mut r = (0.77 + lmr_base * (legal as f64).ln()) as i32;

            // reduce pv nodes less
            r -= i32::from(pv_node);

            // reduce checks less
            r -= i32::from(new.check);

            // reduce passed pawn moves less
            r -= i32::from(mov.moved_pc() == Piece::PAWN && pos.is_passer(mov.from(), pos.stm()));

            // reduce less if next ply had few fail highs
            r -= i32::from(eng.plied[eng.ply].cutoffs < 4);

            // reduce more/less based on history score
            if ms <= MoveScore::HISTORY_MAX {
                r -= ms / 8192
            }

            // don't accidentally extend
            r.max(0)
        } else {
            0
        };

        let pre_nodes = eng.nodes + eng.qnodes;
        eng.plied[eng.ply].played = mov;

        // pvs
        let score = if legal == 1 {
            -pvs(&new, eng, -beta, -alpha, depth + ext - 1, false)
        } else {
            let zw = -pvs(&new, eng, -alpha - 1, -alpha, depth - 1 - reduce, true);

            if zw > alpha && (pv_node || reduce > 0) {
                -pvs(&new, eng, -beta, -alpha, depth - 1, false)
            } else {
                zw
            }
        };

        if eng.ply == 1 {
            eng.ntable.update(mov, eng.nodes + eng.qnodes - pre_nodes);
        }

        best_score = best_score.max(score);

        // improve alpha
        if score <= alpha {
            continue;
        }

        best_move = mov;
        alpha = score;
        bound = Bound::EXACT;

        // update pv line
        if pv_node {
            let sub_line = eng.plied[eng.ply].pv_line;
            let line = &mut eng.plied[eng.ply - 1].pv_line;
            line.copy_in(mov, &sub_line);
        }

        // beta cutoff
        if score < beta {
            continue;
        }

        bound = Bound::LOWER;
        eng.plied[eng.ply - 1].cutoffs += 1;

        // quiet cutoffs pushed to tables
        if mov.is_noisy() || eng.stop_is_set() {
            break;
        }

        eng.plied.push_killer(mov, eng.ply);

        let bonus = 1600.min(350 * (depth - 1));
        eng.htable.push(mov, prevs, pos.stm(), bonus, threats);
        for &quiet in quiets_tried.iter().take(quiets_tried.len() - 1) {
            eng.htable.push(quiet, prevs, pos.stm(), -bonus, threats)
        }

        break;
    }

    eng.pop();

    // end of node shenanigans
    if eng.stop_is_set() {
        return 0;
    }

    if eng.ply == 0 {
        eng.best_move = best_move;
    }

    if legal == 0 {
        return i32::from(pos.check) * (eng.ply - Score::MAX);
    }

    eng.tt
        .push(hash, best_move, depth as i8, bound, best_score, eng.ply);

    best_score
}
