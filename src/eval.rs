use super::consts::*;
use super::position::*;
use super::lsb;

const SIDE_FACTOR: [i16; 3] = [1, -1, 0];
macro_rules! taper {($p:expr, $mg:expr, $eg:expr) => {(($p * $mg as i32 + (TPHASE - $p) * $eg as i32) / TPHASE) as i16}}

pub fn eval() -> i16 {
    let (phase, mg, eg) = calc();
    unsafe { SIDE_FACTOR[POS.side_to_move] * (taper!(phase, mg[0], eg[0]) - taper!(phase, mg[1], eg[1])) }
}

pub fn calc() -> (i32, [i16; 2], [i16; 2]) {
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut p = 0;
    for (i, side) in unsafe{POS.sides.iter().enumerate()} {
        for j in 0..6 {
            let count = (unsafe{POS.pieces[j]} & side).count_ones() as i16;
            p += PHASE_VALS[j] * count;
            mg[i] += MG_PC_VALS[j] * count;
            eg[i] += EG_PC_VALS[j] * count;
        }
    }
    let phase = std::cmp::min(p as i32, TPHASE);
    (phase, mg, eg)
}

pub fn is_in_check() -> bool {
    unsafe {
    let king_idx = lsb!(POS.pieces[KING] & POS.sides[POS.side_to_move]) as usize;
    is_square_attacked(king_idx, POS.side_to_move, POS.sides[0] | POS.sides[1])
    }
}