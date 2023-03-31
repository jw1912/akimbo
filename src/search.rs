use std::{cmp::{max, min}, time::Instant};
use super::{consts::*, position::{Move, Position}, movegen::{MoveList, ScoreList}, tables::{HashTable, KillerTable}};

pub struct Timer(Instant, pub u128);
impl Default for Timer {
    fn default() -> Self {
        Timer(Instant::now(), 1000)
    }
}

#[derive(Default)]
pub struct Engine {
    pub pos: Position,
    pub hash_table: HashTable,
    pub timing: Timer,
    killer_table: KillerTable,
    nodes: u64,
    ply: i16,
    abort: bool,
}

impl ScoreList {
    #[inline(always)]
    fn push(&mut self, score: i16) {
        self.list[self.len] = score;
        self.len += 1;
    }
}

impl Engine {
    fn reset(&mut self) {
        self.timing.0 = Instant::now();
        self.nodes = 0;
        self.ply = 0;
        self.abort = false;
        self.killer_table = KillerTable::default();
    }

    fn score(&self, moves: &MoveList, hash_move: Move) -> ScoreList {
        let mut scores = ScoreList::uninit();
        let killers = self.killer_table.0[self.ply as usize];
        for i in 0..moves.len {
            scores.push({
                let m = moves.list[i];
                if m == hash_move {30000}
                else if m.flag & 4 > 0 {self.mvv_lva(m)}
                else if m.flag & 8 > 0 {950 + i16::from(m.flag & 7)}
                else if killers.contains(&m) {KILLER}
                else {0}
            })
        }
        scores
    }

    fn score_caps(&self, caps: &MoveList) -> ScoreList {
        let mut scores = ScoreList::uninit();
        for i in 0..caps.len {scores.push(self.mvv_lva(caps.list[i]))}
        scores
    }

    fn mvv_lva(&self, m: Move) -> i16 {
        let mpc = self.pos.get_pc(1 << m.from);
        let cpc = self.pos.get_pc(1 << m.to);
        MVV_LVA[cpc][mpc]
    }

    pub fn lazy_eval(&self) -> i16 {
        let score = self.pos.state.pst;
        let p = min(self.pos.phase as i32, TPHASE);
        SIDE[usize::from(self.pos.c)] * ((p * score.0 as i32 + (TPHASE - p) * score.1 as i32) / TPHASE) as i16
    }
}

pub fn go(eng: &mut Engine) {
    eng.reset();
    let mut best_move = Move::default();
    let in_check: bool = eng.pos.in_check();
    for d in 1..=64 {
        let mut pv_line = Vec::with_capacity(d as usize);
        let score: i16 = pvs(eng, -MAX, MAX, d, in_check, false, &mut pv_line);
        if eng.abort {break}
        best_move = pv_line[0];
        let (stype, sval): (&str, i16) = if score.abs() >= MATE {
            ("mate", if score < 0 {score.abs() - MAX} else {MAX - score + 1} / 2)
        } else {("cp", score)};
        let t = eng.timing.0.elapsed();
        let nps: u32 = ((eng.nodes as f64) / t.as_secs_f64()) as u32;
        let pv_str: String = pv_line.iter().map(|&m| Move::to_uci(m)).collect::<String>();
        println!("info depth {d} score {stype} {sval} time {} nodes {} nps {nps} pv {pv_str}", t.as_millis(), eng.nodes);
    }
    println!("bestmove {}", Move::to_uci(best_move));
}

fn qs(eng: &mut Engine, mut a: i16, b: i16) -> i16 {
    eng.nodes += 1;
    let mut e = eng.lazy_eval();
    if e >= b {return e}
    a = max(a, e);
    let mut caps = eng.pos.gen::<CAPTURES>();
    let mut scores = eng.score_caps(&caps);
    while let Some((m, _)) = caps.pick(&mut scores) {
        if eng.pos.r#do(m) { continue }
        e = max(e, -qs(eng, -b, -a));
        eng.pos.undo();
        if e >= b { break }
        a = max(a, e);
    }
    e
}

fn pvs(eng: &mut Engine, mut a: i16, mut b: i16, mut d: i8, in_check: bool, null: bool, line: &mut Vec<Move>) -> i16 {
    if eng.abort { return 0 }
    if eng.nodes & 1023 == 0 && eng.timing.0.elapsed().as_millis() >= eng.timing.1 {
        eng.abort = true;
        return 0
    }

    if eng.pos.state.hfm >= 100 || eng.pos.repetition_draw(2 + u8::from(eng.ply == 0)) || eng.pos.material_draw() { return 0 }
    let pv = b > a + 1;
    a = max(a, -MAX + eng.ply);
    b = min(b, MAX - eng.ply - 1);
    if a >= b { return a }
    d += i8::from(in_check);
    if d <= 0 || eng.ply == MAX_PLY { return qs(eng, a, b) }

    let hash = eng.pos.hash();
    let mut bm = Move::default();
    let mut write = true;
    if let Some(res) = eng.hash_table.probe(hash, eng.ply) {
        write = d > res.depth;
        bm = Move::from_short(res.best_move, &eng.pos);
        if eng.ply > 0 && !write && match res.bound {
            LOWER => res.score >= b,
            UPPER => res.score <= a,
            EXACT => !pv, // want nice pv lines
            _ => false,
        } { return res.score }
    }

    if !pv && !in_check && b.abs() < MATE {
        let eval = eng.lazy_eval();

        // reverse futility pruning
        let m = eval - 120 * i16::from(d);
        if d <= 8 && m >= b { return m }

        // null move pruning
        if null && d >= 3 && eng.pos.phase >= 6 && eval >= b {
            let enp = eng.pos.do_null();
            let nw = -pvs(eng, -a - 1, -a, d - 3, false, false, &mut Vec::new());
            eng.pos.undo_null(enp);
            if nw >= b {return nw}
        }
    }

    eng.nodes += 1;
    eng.ply += 1;
    let lmr = d > 2 && eng.ply > 1 && !in_check;
    let mut moves = eng.pos.gen::<ALL>();
    let mut scores = eng.score(&moves, bm);
    let (mut legal, mut eval, mut bound) = (0, -MAX, UPPER);
    let mut sline = Vec::new();
    while let Some((m, ms)) = moves.pick(&mut scores) {
        if eng.pos.r#do(m) { continue }
        legal += 1;
        let check = eng.pos.in_check();

        // late move reductions
        let r = i8::from(lmr && !check && legal > 2 && ms < KILLER);

        sline.clear();
        let score = if legal == 1 {
            -pvs(eng, -b, -a, d - 1, check, false, &mut sline)
        } else {
            let zw = -pvs(eng, -a - 1, -a, d - 1 - r, check, true, &mut sline);
            if (a != b - 1 || r > 0) && zw > a {
                -pvs(eng, -b, -a, d - 1, check, false, &mut sline)
            } else { zw }
        };
        eng.pos.undo();

        if score > eval {
            eval = score;
            bm = m;
            if score > a {
                a = score;
                bound = EXACT;
                line.clear();
                line.push(m);
                line.append(&mut sline);
                if score >= b {
                    bound = LOWER;
                    if ms <= KILLER {eng.killer_table.push(m, eng.ply)}
                    break
                }
            }
        }
    }
    eng.ply -= 1;
    if legal == 0 { return i16::from(in_check) * (-MAX + eng.ply) }
    if write && !eng.abort { eng.hash_table.push(hash, bm, d, bound, eval, eng.ply) }
    eval
}