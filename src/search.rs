use std::{
    fmt::Write,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::Instant,
};

// used for displaying accurate node counts when multithreading
static DISPLAY_NODES: AtomicU64 = AtomicU64::new(0);

use super::{
    consts::{Bound, MoveScore, Score},
    moves::{Move, MoveList},
    position::Position,
    tables::NodeTable,
    thread::ThreadData,
    tunable_params,
};

tunable_params! {
    nmp_base_reduction = 3, 1, 5;
    nmp_depth_divisor = 3, 1, 8;
    nmp_eval_divisor = 200, 50, 800;
    nmp_eval_max = 3, 0, 8;
    nmp_min_verif_depth = 12, 8, 20;
    nmp_verif_frac = 12, 1, 16;
    rfp_margin = 80, 20, 200;
    razor_margin = 400, 200, 800;
    lmr_base = 77, 0, 512;
    lmr_divisor = 267, 128, 512;
    fp_base = 160, 80, 400;
    fp_margin = 80, 20, 200;
    hist_max = 1600, 800, 4000;
    hist_mul = 350, 100, 500;
}

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
    DISPLAY_NODES.store(0, Relaxed);

    // reset engine
    eng.store_stop(false);
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

    let mut accs = Default::default();
    pos.refresh(&mut accs);
    eng.plied[0].accumulators = accs;

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
            let display_nodes = DISPLAY_NODES.load(Relaxed);
            let nps = (1000.0 * display_nodes as f64 / t as f64) as u32;
            let pv_line = &eng.plied[0].pv_line;
            let pv = pv_line.iter().fold(String::new(), |mut pv_str, mov| {
                write!(&mut pv_str, "{} ", mov.to_uci(&eng.castling)).unwrap();
                pv_str
            });
            println!(
                "info depth {d} seldepth {} {score} time {t} nodes {display_nodes} nps {nps:.0} pv {pv}",
                eng.seldepth
            );

            let frac = eng.ntable.get(best_move) as f64 / nodes as f64;
            if eng.timing.elapsed().as_millis() as f64
                >= soft_bound * if d > 8 { (1.5 - frac) * 1.35 } else { 1.0 }
            {
                eng.store_stop(true);
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
    let mut delta = 16;
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

    let hash = pos.hash();
    let mut eval = pos.eval(&eng.plied[eng.ply].accumulators);

    // probe hash table for cutoff
    if let Some(entry) = eng.tt.probe(hash, eng.ply) {
        let tt_score = entry.score();
        let bound = entry.bound();
        if match bound {
            Bound::LOWER => tt_score >= beta,
            Bound::UPPER => tt_score <= alpha,
            _ => true,
        } {
            return tt_score;
        }

        // use tt score instead of static eval
        if !((eval > tt_score && bound == Bound::LOWER)
            || (eval < tt_score && bound == Bound::UPPER))
        {
            eval = tt_score;
        }
    }

    // stand-pat
    if eval >= beta {
        return eval;
    }

    alpha = alpha.max(eval);

    let mut caps = pos.movegen::<false>(&eng.castling);
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

        let after = pos.key_after(hash, mov);
        eng.tt.prefetch(after);

        let mut new = *pos;
        if new.make(mov, &eng.castling) {
            continue;
        }

        eng.update_accumulators(&new);

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

    if eng.nodes & 1023 == 0 {
        DISPLAY_NODES.fetch_add(1024, Relaxed);

        if eng.timing.elapsed().as_millis() >= eng.max_time
            || eng.nodes + eng.qnodes >= eng.max_nodes
        {
            eng.store_stop(true);
            return 0;
        }
    }

    let hash = pos.hash();
    let is_root = eng.ply == 0;

    // clear pv line
    eng.plied[eng.ply].pv_line.clear();

    if !is_root {
        // draw detection
        if pos.draw() || eng.repetition(pos, hash, false) {
            return Score::DRAW;
        }

        // mate distance pruning
        alpha = alpha.max(eng.ply - Score::MAX);
        beta = beta.min(Score::MAX - eng.ply - 1);
        if alpha >= beta {
            return alpha;
        }

        // check extensions
        depth += i32::from(pos.check);
    }

    // drop into quiescence search
    if depth <= 0 || eng.ply == 95 {
        return qs(pos, eng, alpha, beta);
    }

    let pv_node = beta > alpha + 1;
    let s_mov = eng.plied[eng.ply].singular;
    let singular = s_mov != Move::NULL;
    let pc_beta = beta + 256;
    let static_eval = pos.eval(&eng.plied[eng.ply].accumulators);

    let mut eval = static_eval;
    let mut tt_move = Move::NULL;
    let mut tt_score = -Score::MAX;
    let mut try_singular = !is_root && !singular && depth >= 8;
    let mut can_probcut = true;

    // probing hash table
    if let Some(entry) = eng.tt.probe(hash, eng.ply) {
        let bound = entry.bound();
        let depth_cond = entry.depth() >= depth - 3;

        tt_move = entry.best_move(pos);
        tt_score = entry.score();
        try_singular &= depth_cond && bound != Bound::UPPER && tt_score.abs() < Score::MATE;
        can_probcut = !(depth_cond && tt_score < pc_beta);

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
    eng.plied[eng.ply].eval = static_eval;
    let improving = eng.ply > 1 && static_eval > eng.plied[eng.ply - 2].eval;

    // pruning
    let can_prune = !pv_node && !pos.check;
    if can_prune && beta.abs() < Score::MATE {
        // reverse futility pruning
        let improving_divisor = if improving { 2 } else { 1 };
        if depth <= 8 && eval >= beta + rfp_margin() * depth / improving_divisor {
            return eval;
        }

        // razoring
        if depth <= 2 && eval + razor_margin() * depth < alpha {
            let qeval = qs(pos, eng, alpha, beta);

            if qeval < alpha {
                return qeval;
            }
        }

        // null move pruning
        if null
            && eng.ply >= eng.min_nmp_ply
            && depth >= 3
            && pos.has_non_pk(pos.stm())
            && eval >= beta
        {
            let r = nmp_base_reduction()
                + depth / nmp_depth_divisor()
                + nmp_eval_max().min((eval - beta) / nmp_eval_divisor())
                + i32::from(improving);

            eng.push(hash);
            eng.plied[eng.ply].accumulators = eng.plied[eng.ply - 1].accumulators;
            eng.plied[eng.ply].played = Move::NULL;

            let mut new = *pos;
            new.make_null();

            let nw = -pvs(&new, eng, -beta, -alpha, depth - r, false);

            eng.pop();

            if nw >= beta {
                // don't bother to verify on low depths
                if depth < nmp_min_verif_depth() || eng.min_nmp_ply > 0 {
                    return if nw > Score::MATE { beta } else { nw };
                }

                eng.min_nmp_ply = eng.ply + (depth - r) * nmp_verif_frac() / 16;

                let verif = pvs(pos, eng, beta - 1, beta, depth - r, false);

                eng.min_nmp_ply = 0;

                if verif >= beta {
                    return verif;
                }
            }
        }
    }

    // internal iterative reduction
    depth -= i32::from(depth >= 4 && tt_move == Move::NULL);

    // probcut
    if can_prune && depth > 4 && beta.abs() < Score::MATE && can_probcut {
        let mut caps = pos.movegen::<false>(&eng.castling);
        let mut scores = [0; 252];

        caps.iter()
            .enumerate()
            .for_each(|(i, &cap)| scores[i] = mvv_lva(cap, pos));

        eng.push(hash);

        while let Some((mov, _)) = caps.pick(&mut scores) {
            // static exchange eval pruning
            if !pos.see(mov, 1) {
                continue;
            }

            let mut new = *pos;
            if new.make(mov, &eng.castling) {
                continue;
            }

            eng.update_accumulators(&new);

            eng.nodes += 1;

            let mut pc_score = -qs(&new, eng, -pc_beta, -pc_beta + 1);

            if pc_score >= pc_beta {
                pc_score = -pvs(&new, eng, -pc_beta, -pc_beta + 1, depth - 4, false)
            }

            if pc_score >= pc_beta {
                eng.pop();
                eng.tt
                    .push(hash, mov, depth as i8 - 3, Bound::LOWER, pc_beta, eng.ply);

                return pc_beta;
            }
        }

        eng.pop();
    }

    // generating moves
    let mut moves = pos.movegen::<true>(&eng.castling);

    let prev = eng.plied.prev_move(eng.ply, 1);
    let prevs = [prev, eng.plied.prev_move(eng.ply, 2)];

    let threats = pos.threats();
    let killer = eng.plied[eng.ply].killer;

    // scoring moves
    let mut scores = [0; 252];
    for (i, &mov) in moves.iter().enumerate() {
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
    }

    let mut legal = 0;
    let mut bound = Bound::UPPER;
    let mut best_score = -Score::MAX;
    let mut best_move = tt_move;
    let mut quiets_tried = MoveList::ZEROED;

    let can_lmr = depth > 1 && !pos.check;
    let lmr_base = f64::from(lmr_base()) / 100.0;
    let lmr_depth = (depth as f64).ln() / (f64::from(lmr_divisor()) / 100.0);

    #[cfg(not(feature = "datagen"))]
    let can_fp = !singular && depth < 6;

    #[cfg(not(feature = "datagen"))]
    let lmp_margin = 2 + depth * depth / if improving { 1 } else { 2 };

    #[cfg(not(feature = "datagen"))]
    let fp_margin = eval + fp_base() + fp_margin() * depth * depth;

    eng.push(hash);
    eng.plied[eng.ply].dbl_exts = eng.plied[eng.ply - 1].dbl_exts;

    while let Some((mov, ms)) = moves.pick(&mut scores) {
        // move is singular in a singular search
        if mov == s_mov {
            continue;
        }

        // pre-move pruning
        #[cfg(not(feature = "datagen"))]
        if can_prune && best_score.abs() < Score::MATE {
            // late move pruning
            if ms < MoveScore::KILLER {
                // late move pruning
                if legal > lmp_margin {
                    break;
                }

                // futility pruning
                if can_fp && alpha < Score::MATE && fp_margin <= alpha {
                    break;
                }

                // history pruning
                if depth < 3 && ms < -1024 * depth {
                    break;
                }
            }

            // static exchange eval pruning
            let margin = if mov.is_capture() { -90 } else { -50 };
            if depth < 7 && ms < MoveScore::CAPTURE && !pos.see(mov, margin * depth) {
                continue;
            }
        }

        // prefetch new tt probe ahead of time
        let after = pos.key_after(hash, mov);
        eng.tt.prefetch(after);

        // make move and skip if not legal
        let mut new = *pos;
        if new.make(mov, &eng.castling) {
            continue;
        }

        // update accumulators based on new position
        eng.update_accumulators(&new);

        new.check = new.in_check();
        eng.nodes += 1;
        legal += 1;

        if !mov.is_noisy() {
            quiets_tried.add(mov);
        }

        let mut extend = 0;
        let mut reduce = 0;

        // singular extensions
        if try_singular && mov == tt_move {
            let s_beta = tt_score - depth * 2;

            let curr_accs = eng.plied[eng.ply].accumulators;
            eng.pop();
            eng.plied[eng.ply].singular = mov;

            let s_score = pvs(pos, eng, s_beta - 1, s_beta, (depth - 1) / 2, false);

            eng.plied[eng.ply].singular = Move::NULL;
            eng.push(hash);
            eng.plied[eng.ply].accumulators = curr_accs;

            if s_score < s_beta {
                // tt move is singular, extend
                extend = 1;

                // double extension
                if !pv_node && s_score < s_beta - 25 && eng.plied[eng.ply].dbl_exts < 5 {
                    eng.plied[eng.ply].dbl_exts += 1;
                    extend += 1
                }
            } else if tt_score >= beta || (tt_score <= alpha && null) {
                // negative extension
                extend = -1
            }
        }

        // reductions
        if can_lmr && ms < MoveScore::KILLER {
            // late move reductions - Viridithas values used
            reduce = (lmr_base + lmr_depth * (legal as f64).ln()) as i32;

            // reduce pv nodes less
            reduce -= i32::from(pv_node);

            // reduce checks less
            reduce -= i32::from(new.check);

            // reduce less if next ply had few fail highs
            reduce -= i32::from(eng.plied[eng.ply].cutoffs < 4);

            // reduce more/less based on history score
            if ms <= MoveScore::HISTORY_MAX {
                reduce -= ms / 8192
            }

            // don't accidentally extend
            reduce = reduce.max(0)
        };

        let pre_nodes = eng.nodes + eng.qnodes;
        eng.plied[eng.ply].played = mov;

        // pvs
        let score = if legal == 1 {
            -pvs(&new, eng, -beta, -alpha, depth + extend - 1, false)
        } else {
            let mut zw = -pvs(&new, eng, -alpha - 1, -alpha, depth - 1 - reduce, true);

            if zw > alpha && (pv_node || reduce > 0) {
                zw = -pvs(&new, eng, -beta, -alpha, depth - 1, false)
            }

            zw
        };

        // update node count table for node tm
        if is_root {
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

        if quiets_tried.len() > 1 || depth > 2 {
            let bonus = hist_max().min(hist_mul() * (depth - 1));
            eng.htable.push(mov, prevs, pos.stm(), bonus, threats);

            for &quiet in quiets_tried.iter().take(quiets_tried.len() - 1) {
                eng.htable.push(quiet, prevs, pos.stm(), -bonus, threats)
            }
        }

        break;
    }

    eng.pop();

    // end of node shenanigans
    if eng.stop_is_set() {
        return 0;
    }

    // set best move at root
    if is_root {
        eng.best_move = best_move;
    }

    // checkmate / stalemate
    if legal == 0 {
        return i32::from(pos.check) * (eng.ply - Score::MAX);
    }

    // push new entry to hash table
    eng.tt
        .push(hash, best_move, depth as i8, bound, best_score, eng.ply);

    best_score
}
