use std::{
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
    nmp_base_reduction = 5, 1, 5, 1;
    nmp_depth_divisor = 5, 1, 8, 1;
    nmp_eval_divisor = 198, 50, 800, 100;
    nmp_eval_max = 6, 0, 8, 1;
    nmp_min_verif_depth = 17, 8, 20, 4;
    nmp_verif_frac = 12, 1, 16, 4;
    rfp_margin = 94, 20, 200, 40;
    razor_margin = 407, 200, 800, 100;
    lmr_base = 48, 0, 512, 32;
    lmr_divisor = 248, 128, 512, 64;
    fp_base = 188, 80, 400, 40;
    fp_margin = 35, 20, 200, 40;
    hist_bonus_max = 1827, 800, 4000, 200;
    hist_bonus_mul = 375, 100, 500, 100;
    hist_bonus_offset = 141, 0, 1000, 100;
    hist_malus_max = 1192, 800, 4000, 200;
    hist_malus_mul = 396, 100, 500, 100;
    hist_malus_offset = 8, 0, 1000, 100;
    rfp_depth = 8, 4, 16, 2;
    razor_depth = 0, 0, 10, 2;
    nmp_depth = 2, 1, 8, 2;
    iir_depth = 4, 1, 12, 2;
    pc_depth = 5, 1, 12, 2;
    see_cap_margin = 148, 30, 150, 30;
    see_quiet_margin = 64, 10, 150, 30;
    se_margin = 1, 0, 6, 1;
    hist_prune_depth = 6, 0, 8, 1;
    hist_prune_margin = 1682, 512, 2048, 256;
}

fn mvv_lva(mov: Move, pos: &Position) -> i32 {
    8 * pos.get_pc(1 << mov.to()) as i32 - mov.moved_pc() as i32
}

pub fn go(
    start: &Position,
    td: &mut ThreadData,
    main_thread: bool,
    max_depth: i32,
    soft_bound: f64,
    soft_nodes: u64,
) -> (Move, i32) {
    DISPLAY_NODES.store(0, Relaxed);

    // reset engine
    td.store_stop(false);
    td.ntable = NodeTable::default();
    td.plied.clear();
    td.timing = Instant::now();
    td.nodes = 0;
    td.qnodes = 0;
    td.ply = 0;
    td.best_move = Move::NULL;
    td.seldepth = 0;

    let mut best_move = Move::NULL;
    let mut eval = 0;
    let mut score = 0;
    td.plied[0].in_check = start.in_check();

    // iterative deepening loop
    for d in 1..=max_depth {
        eval = if d < 7 {
            pvs(start, td, -Score::MAX, Score::MAX, d, false)
        } else {
            aspiration(start, td, eval, d, &mut best_move)
        };

        if td.stop_is_set() {
            break;
        }

        best_move = td.best_move;
        score = eval;

        if main_thread {
            // UCI output
            print!("info depth {d} seldepth {} ", td.seldepth);

            // format mate scores if appropriate
            if eval.abs() >= Score::MATE {
                let mate_in = if eval < 0 {
                    eval.abs() - Score::MAX
                } else {
                    Score::MAX - eval + 1
                };

                print!("score mate {} ", mate_in / 2);
            } else {
                print!("score cp {eval} ");
            };

            let time = td.timer();
            let nodes = DISPLAY_NODES.load(Relaxed);
            let nps = (1000.0 * nodes as f64 / time as f64) as u32;

            print!("time {time} nodes {nodes} nps {nps} pv");

            // output pv line
            for mov in td.plied[0].pv_line.iter() {
                print!(" {}", mov.to_uci(&td.castling));
            }

            println!();

            let frac = td.ntable.get(best_move) as f64 / td.nodes() as f64;
            let multiplier = if d > 8 { (1.5 - frac) * 1.35 } else { 1.0 };

            // soft timeout
            if time as f64 >= soft_bound * multiplier {
                td.store_stop(true);
                break;
            }
        }

        // soft node limit
        if td.nodes() > soft_nodes {
            break;
        }
    }

    (best_move, score)
}

fn aspiration(
    pos: &Position,
    td: &mut ThreadData,
    mut score: i32,
    max_depth: i32,
    best_move: &mut Move,
) -> i32 {
    let mut delta = 16;
    let mut alpha = (-Score::MAX).max(score - delta);
    let mut beta = Score::MAX.min(score + delta);
    let mut depth = max_depth;

    loop {
        score = pvs(pos, td, alpha, beta, depth, false);

        if td.stop_is_set() {
            return 0;
        }

        if score <= alpha {
            beta = (alpha + beta) / 2;
            alpha = (-Score::MAX).max(alpha - delta);
            depth = max_depth;
        } else if score >= beta {
            beta = Score::MAX.min(beta + delta);
            *best_move = td.best_move;
            depth -= 1;
        } else {
            return score;
        }

        delta *= 2;
    }
}

fn qs(pos: &Position, td: &mut ThreadData, mut alpha: i32, beta: i32) -> i32 {
    td.seldepth = td.seldepth.max(td.ply);

    let hash = pos.hash();
    let mut eval = td
        .chtable
        .correct_evaluation(pos, pos.eval());

    // probe hash table for cutoff
    if let Some(entry) = td.tt.probe(hash, td.ply) {
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

    let mut caps = pos.movegen::<false>(&td.castling);
    let mut scores = [0; 252];

    caps.iter()
        .enumerate()
        .for_each(|(i, &cap)| scores[i] = mvv_lva(cap, pos));

    let mut best_move = Move::NULL;
    let mut bound = Bound::UPPER;

    td.ply += 1;

    while let Some((mov, _)) = caps.pick(&mut scores) {
        // static exchange eval pruning
        if !pos.see(mov, 1) {
            continue;
        }

        let after = pos.key_after(hash, mov);
        td.tt.prefetch(after);

        let mut new = *pos;
        if new.make(mov, &td.castling) {
            continue;
        }

        td.qnodes += 1;

        let score = -qs(&new, td, -beta, -alpha);

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

    td.ply -= 1;

    td.tt.push(hash, best_move, 0, bound, eval, td.ply);

    eval
}

fn pvs(
    pos: &Position,
    td: &mut ThreadData,
    mut alpha: i32,
    mut beta: i32,
    mut depth: i32,
    null: bool,
) -> i32 {
    // stopping search
    if td.stop_is_set() {
        return 0;
    }

    if td.nodes & 1023 == 0 {
        DISPLAY_NODES.fetch_add(1024, Relaxed);

        if td.timer() >= td.max_time || td.nodes() >= td.max_nodes {
            td.store_stop(true);
            return 0;
        }
    }

    let hash = pos.hash();
    let is_root = td.ply == 0;
    let in_check = td.plied[td.ply].in_check;

    // clear pv line
    td.plied[td.ply].pv_line.clear();

    if !is_root {
        // draw detection
        if pos.is_draw() || td.repetition(pos, hash, false) {
            return Score::DRAW;
        }

        // mate distance pruning
        alpha = alpha.max(td.ply - Score::MAX);
        beta = beta.min(Score::MAX - td.ply - 1);
        if alpha >= beta {
            return alpha;
        }

        // check extensions
        depth += i32::from(in_check);
    }

    // drop into quiescence search
    if depth <= 0 || td.ply == 95 {
        return qs(pos, td, alpha, beta);
    }

    let pv_node = beta > alpha + 1;
    let s_mov = td.plied[td.ply].singular;
    let singular = s_mov != Move::NULL;
    let pc_beta = beta + 256;
    let mut static_eval = pos.eval();

    if !singular {
        static_eval = td.chtable.correct_evaluation(pos, static_eval);
    }

    let mut eval = static_eval;
    let mut tt_move = Move::NULL;
    let mut tt_score = -Score::MAX;
    let mut try_singular = !is_root && !singular && depth >= 8;
    let mut can_probcut = true;

    // probing hash table
    if let Some(entry) = td.tt.probe(hash, td.ply) {
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
    td.plied[td.ply].eval = static_eval;
    let improving = td.ply > 1 && static_eval > td.plied[td.ply - 2].eval;

    // pruning
    let can_prune = !pv_node && !in_check;
    if can_prune && beta.abs() < Score::MATE {
        // reverse futility pruning
        let improving_divisor = if improving { 2 } else { 1 };
        if depth <= rfp_depth() && eval >= beta + rfp_margin() * depth / improving_divisor {
            return eval;
        }

        // razoring
        if depth <= razor_depth() && eval + razor_margin() * depth < alpha {
            let qeval = qs(pos, td, alpha, beta);

            if qeval < alpha {
                return qeval;
            }
        }

        // null move pruning
        if null
            && td.ply >= td.min_nmp_ply
            && depth >= nmp_depth()
            && pos.has_non_pk(pos.stm())
            && eval >= beta
        {
            let r = nmp_base_reduction()
                + depth / nmp_depth_divisor()
                + nmp_eval_max().min((eval - beta) / nmp_eval_divisor())
                + i32::from(improving);

            td.push(hash);
            td.plied[td.ply].played = Move::NULL;
            td.plied[td.ply].in_check = false;

            let mut new = *pos;
            new.make_null();

            let nw = -pvs(&new, td, -beta, -alpha, depth - r, false);

            td.pop();

            if nw >= beta {
                // don't bother to verify on low depths
                if depth < nmp_min_verif_depth() || td.min_nmp_ply > 0 {
                    return if nw > Score::MATE { beta } else { nw };
                }

                td.min_nmp_ply = td.ply + (depth - r) * nmp_verif_frac() / 16;

                let verif = pvs(pos, td, beta - 1, beta, depth - r, false);

                td.min_nmp_ply = 0;

                if verif >= beta {
                    return verif;
                }
            }
        }
    }

    // internal iterative reduction
    depth -= i32::from(depth >= iir_depth() && tt_move == Move::NULL);

    // probcut
    if can_prune && depth > pc_depth() && beta.abs() < Score::MATE && can_probcut {
        let mut caps = pos.movegen::<false>(&td.castling);
        let mut scores = [0; 252];

        caps.iter()
            .enumerate()
            .for_each(|(i, &cap)| scores[i] = mvv_lva(cap, pos));

        td.push(hash);

        // not correct, try removing?
        td.plied[td.ply].in_check = false;

        while let Some((mov, _)) = caps.pick(&mut scores) {
            // static exchange eval pruning
            if !pos.see(mov, 1) {
                continue;
            }

            let mut new = *pos;
            if new.make(mov, &td.castling) {
                continue;
            }

            td.nodes += 1;

            let mut pc_score = -qs(&new, td, -pc_beta, -pc_beta + 1);

            if pc_score >= pc_beta {
                pc_score = -pvs(&new, td, -pc_beta, -pc_beta + 1, depth - 4, false)
            }

            if pc_score >= pc_beta {
                td.pop();
                td.tt
                    .push(hash, mov, depth as i8 - 3, Bound::LOWER, pc_beta, td.ply);

                return pc_beta;
            }
        }

        td.pop();
    }

    // generating moves
    let mut moves = pos.movegen::<true>(&td.castling);

    let prev = td.plied.prev_move(td.ply, 1);
    let prevs = [prev, td.plied.prev_move(td.ply, 2)];

    let threats = pos.threats();
    let killer = td.plied[td.ply].killer;

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
            td.htable.get_score(pos.stm(), mov, prevs, threats)
        }
    }

    let mut legal = 0;
    let mut bound = Bound::UPPER;
    let mut best_score = -Score::MAX;
    let mut best_move = tt_move;
    let mut quiets_tried = MoveList::ZEROED;

    let can_lmr = depth > 1 && !in_check;
    let lmr_base = f64::from(lmr_base()) / 100.0;
    let lmr_depth = (depth as f64).ln() / (f64::from(lmr_divisor()) / 100.0);
    let can_fp = !singular && depth < 6;
    let lmp_margin = 2 + depth * depth / if improving { 1 } else { 2 };
    let fp_margin = eval + fp_base() + fp_margin() * depth * depth;

    td.push(hash);
    td.plied[td.ply].dbl_exts = td.plied[td.ply - 1].dbl_exts;

    while let Some((mov, ms)) = moves.pick(&mut scores) {
        // move is singular in a singular search
        if mov == s_mov {
            continue;
        }

        // pre-move pruning
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
                if depth < hist_prune_depth() && ms < -hist_prune_margin() * depth {
                    break;
                }
            }

            // static exchange eval pruning
            let margin = if mov.is_capture() {
                -see_cap_margin()
            } else {
                -see_quiet_margin()
            };
            if depth < 7 && ms < MoveScore::CAPTURE && !pos.see(mov, margin * depth) {
                continue;
            }
        }

        // prefetch new tt probe ahead of time
        let after = pos.key_after(hash, mov);
        td.tt.prefetch(after);

        // make move and skip if not legal
        let mut new = *pos;
        if new.make(mov, &td.castling) {
            continue;
        }

        td.nodes += 1;
        legal += 1;

        if !mov.is_noisy() {
            quiets_tried.add(mov);
        }

        let mut extend = 0;
        let mut reduce = 0;

        // singular extensions
        if try_singular && mov == tt_move {
            let s_beta = tt_score - depth * se_margin();

            td.pop();
            td.plied[td.ply].singular = mov;

            let s_score = pvs(pos, td, s_beta - 1, s_beta, (depth - 1) / 2, false);

            td.plied[td.ply].singular = Move::NULL;
            td.push(hash);

            if s_score < s_beta {
                // tt move is singular, extend
                extend = 1;

                // double extension
                if !pv_node && s_score < s_beta - 25 && td.plied[td.ply].dbl_exts < 5 {
                    td.plied[td.ply].dbl_exts += 1;
                    extend += 1
                }
            } else if tt_score >= beta || (tt_score <= alpha && null) {
                // negative extension
                extend = -1
            }
        }

        let new_in_check = new.in_check();

        // reductions
        if can_lmr && ms < MoveScore::KILLER {
            // late move reductions - Viridithas values used
            reduce = (lmr_base + lmr_depth * (legal as f64).ln()) as i32;

            // reduce pv nodes less
            reduce -= i32::from(pv_node);

            // reduce checks less
            reduce -= i32::from(new_in_check);

            // reduce less if next ply had few fail highs
            reduce -= i32::from(td.plied[td.ply].cutoffs < 4);

            // reduce more/less based on history score
            if ms <= MoveScore::HISTORY_MAX {
                reduce -= ms / 8192
            }

            // don't accidentally extend
            reduce = reduce.max(0)
        };

        let pre_nodes = td.nodes();
        td.plied[td.ply].played = mov;
        td.plied[td.ply].in_check = new_in_check;

        // pvs
        let score = if legal == 1 {
            -pvs(&new, td, -beta, -alpha, depth + extend - 1, false)
        } else {
            let mut zw = -pvs(&new, td, -alpha - 1, -alpha, depth - 1 - reduce, true);

            if zw > alpha && (pv_node || reduce > 0) {
                zw = -pvs(&new, td, -beta, -alpha, depth - 1, false)
            }

            zw
        };

        // update node count table for node tm
        if is_root {
            td.ntable.update(mov, td.nodes() - pre_nodes);
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
            let sub_line = td.plied[td.ply].pv_line;
            let line = &mut td.plied[td.ply - 1].pv_line;
            line.copy_in(mov, &sub_line);
        }

        // beta cutoff
        if score < beta {
            continue;
        }

        bound = Bound::LOWER;
        td.plied[td.ply - 1].cutoffs += 1;

        // quiet cutoffs pushed to tables
        if mov.is_noisy() || td.stop_is_set() {
            break;
        }

        td.plied.push_killer(mov, td.ply);

        if quiets_tried.len() > 1 || depth > 2 {
            let bonus = hist_bonus_max().min(hist_bonus_mul() * depth - hist_bonus_offset());
            let malus = hist_malus_max().min(hist_malus_mul() * depth - hist_malus_offset());
            td.htable.push(mov, prevs, pos.stm(), bonus, threats);

            for &quiet in quiets_tried.iter().take(quiets_tried.len() - 1) {
                td.htable.push(quiet, prevs, pos.stm(), -malus, threats)
            }
        }

        break;
    }

    td.pop();

    // end of node shenanigans
    if td.stop_is_set() {
        return 0;
    }

    // set best move at root
    if is_root {
        td.best_move = best_move;
    }

    // checkmate / stalemate
    if legal == 0 {
        return i32::from(in_check) * (td.ply - Score::MAX);
    }

    // update corrhist table
    if !(singular
        || in_check
        || best_move.is_noisy()
        || bound == Bound::LOWER && best_score <= static_eval
        || bound == Bound::UPPER && best_score >= static_eval)
    {
        td.chtable
            .update_correction_history(pos, depth, best_score - static_eval);
    }

    // push new entry to hash table
    td.tt
        .push(hash, best_move, depth as i8, bound, best_score, td.ply);

    best_score
}
