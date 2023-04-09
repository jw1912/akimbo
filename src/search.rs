use std::{cmp::{max, min}, time::Instant};
use super::{consts::*, decl_mut, position::{Move, Position}, movegen::{MoveList, ScoreList}, tables::{HashTable, HistoryTable, KillerTable}};

pub struct Timer(Instant, pub u128);
impl Default for Timer {
    fn default() -> Self {
        Timer(Instant::now(), 1000)
    }
}

#[derive(Default)]
pub struct Engine {
    pub pos: Position,
    pub timing: Timer,
    pub ttable: HashTable,
    pub htable: Box<HistoryTable>,
    ktable: Box<KillerTable>,
    nodes: u64,
    qnodes: u64,
    ply: i16,
    abort: bool,
}

impl Engine {
    fn reset(&mut self) {
        self.timing.0 = Instant::now();
        self.nodes = 0;
        self.qnodes = 0;
        self.ply = 0;
        self.abort = false;
    }

    fn score(&self, moves: &MoveList, hash_move: Move) -> ScoreList {
        let mut scores = ScoreList::uninit();
        let killers = self.ktable.0[self.ply as usize];
        for &m in &moves.list[0..moves.len] {
            scores.add({
                if m == hash_move { HASH }
                else if m.flag == ENP { 2 * MVV_LVA }
                else if m.flag & 4 > 0 { self.mvv_lva(m) }
                else if m.flag & 8 > 0 { PROMOTION + i16::from(m.flag & 7) }
                else if killers.contains(&m) { KILLER }
                else {self.htable.score(self.pos.c, m)}
            })
        }
        scores
    }

    fn score_caps(&self, caps: &MoveList) -> ScoreList {
        let mut scores = ScoreList::uninit();
        for i in 0..caps.len {scores.add(self.mvv_lva(caps.list[i]))}
        scores
    }

    fn mvv_lva(&self, m: Move) -> i16 {
        MVV_LVA * self.pos.get_pc(1 << m.to) as i16 - m.mpc as i16
    }
}

pub fn go(eng: &mut Engine) {
    eng.reset();
    let mut best_move = Move::default();
    let in_check: bool = eng.pos.in_check();

    for d in 1..=64 {
        let mut pv_line = Vec::with_capacity(d as usize);
        let score = search(eng, -MAX, MAX, d, in_check, false, &mut pv_line);
        if eng.abort { break }
        best_move = pv_line[0];

        // UCI output
        let (stype, sval) = if score.abs() >= MATE {
            ("mate", if score < 0 {score.abs() - MAX} else {MAX - score + 1} / 2)
        } else {("cp", score)};
        let t = eng.timing.0.elapsed();
        let nodes = eng.nodes + eng.qnodes;
        let nps = ((nodes as f64) / t.as_secs_f64()) as u32;
        let pv_str = pv_line.iter().map(|&m| m.to_uci()).collect::<String>();
        println!("info depth {d} score {stype} {sval} time {} nodes {nodes} nps {nps} pv {pv_str}", t.as_millis());
    }

    println!("bestmove {}", best_move.to_uci());
    *eng.ktable = Default::default();
    eng.htable.age();
}

fn qsearch(eng: &mut Engine, mut alpha: i16, beta: i16) -> i16 {
    eng.qnodes += 1;
    let mut eval = eng.pos.lazy_eval();
    if eval >= beta { return eval }
    alpha = max(alpha, eval);
    let mut caps = eng.pos.gen::<CAPTURES>();
    let mut scores = eng.score_caps(&caps);
    while let Some((r#move, _)) = caps.pick(&mut scores) {
        if eng.pos.r#do(r#move) { continue }
        eval = max(eval, -qsearch(eng, -beta, -alpha));
        eng.pos.undo();
        if eval >= beta { break }
        alpha = max(alpha, eval);
    }
    eval
}

fn search(eng: &mut Engine, mut alpha: i16, mut beta: i16, mut depth: i8, in_check: bool, null: bool, line: &mut Vec<Move>) -> i16 {
    // stopping search
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.0.elapsed().as_millis() >= eng.timing.1 {
        eng.abort = true;
        return 0
    }

    line.clear();

    // draw detection
    if eng.pos.is_draw(eng.ply) { return 0 }

    // mate distance pruning
    alpha = max(alpha, -MAX + eng.ply);
    beta = min(beta, MAX - eng.ply - 1);
    if alpha >= beta { return alpha }

    // check extensions
    depth += i8::from(in_check);

    if depth <= 0 || eng.ply == MAX_PLY { return qsearch(eng, alpha, beta) }

    eng.nodes += 1;
    let pv_node = beta > alpha + 1;
    let hash = eng.pos.hash();
    decl_mut!(best_move = Move::default(), write = true);

    // probing hash table
    if let Some(res) = eng.ttable.probe(hash, eng.ply) {
        write = depth > res.depth;
        best_move = Move::from_short(res.best_move, &eng.pos);

        // hash score pruning
        if !pv_node && res.depth >= depth && match res.bound {
            LOWER => res.score >= beta,
            UPPER => res.score <= alpha,
            _ => true,
        } { return res.score }
    }

    // pruning
    if !pv_node && !in_check && beta.abs() < MATE {
        let eval = eng.pos.lazy_eval();

        // reverse futility pruning
        let margin = eval - 120 * i16::from(depth);
        if depth <= 8 && margin >= beta { return margin }

        // null move pruning
        if null && depth >= 3 && eng.pos.phase >= 6 && eval >= beta {
            let r = 2 + depth / 3;
            eng.ply += 1;
            eng.pos.r#do_null();
            let nw = -search(eng, -alpha - 1, -alpha, depth - r, false, false, &mut Vec::new());
            eng.pos.undo_null();
            eng.ply -= 1;
            if nw >= beta {
                if nw >= MATE { return beta }
                return nw
            }
        }
    }

    let mut moves = eng.pos.gen::<ALL>();
    let mut scores = eng.score(&moves, best_move);
    let lmr = depth > 1 && eng.ply > 0 && !in_check;
    decl_mut!(legal = 0, eval = -MAX, bound = UPPER, sline = Vec::new());

    eng.ply += 1;
    while let Some((r#move, mscore)) = moves.pick(&mut scores) {
        if eng.pos.r#do(r#move) { continue }
        let check = eng.pos.in_check();
        legal += 1;

        // late move reductions - Viridithas values used
        let reduce = if lmr && !check && mscore < KILLER {
            let lmr = (0.77 + f64::from(depth).ln() * f64::from(legal).ln() / 2.67) as i8;
            if pv_node { max(1, lmr - 1) } else { lmr }
        } else {0};

        // pvs framework
        let score = if legal == 1 {
            -search(eng, -beta, -alpha, depth - 1, check, false, &mut sline)
        } else {
            let zw = -search(eng, -alpha - 1, -alpha, depth - 1 - reduce, check, true, &mut sline);
            if (pv_node || reduce > 0) && zw > alpha {
                -search(eng, -beta, -alpha, depth - 1, check, false, &mut sline)
            } else { zw }
        };
        eng.pos.undo();

        if score > eval {
            eval = score;
            best_move = r#move;
            if score > alpha {
                alpha = score;
                bound = EXACT;
                line.clear();
                line.push(r#move);
                line.append(&mut sline);
                if score >= beta {
                    bound = LOWER;
                    if r#move.flag < CAP {
                        eng.ktable.push(r#move, eng.ply);
                        eng.htable.push(r#move, eng.pos.c, depth);
                    }
                    break
                }
            }
        }
    }
    eng.ply -= 1;

    if legal == 0 { return i16::from(in_check) * (-MAX + eng.ply) }
    if write && !eng.abort { eng.ttable.push(hash, best_move, depth, bound, eval, eng.ply) }
    eval
}