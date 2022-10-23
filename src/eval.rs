use crate::pop;

use super::consts::*;
use super::position::*;
use super::lsb;

macro_rules! taper {($p:expr, $mg:expr, $eg:expr) => {(($p * $mg as i32 + (TPHASE - $p) * $eg as i32) / TPHASE) as i16}}

#[inline(always)]
pub fn eval() -> i16 {
    unsafe {
    let phase = std::cmp::min(POS.state.phase as i32, TPHASE);
    SIDE_FACTOR[POS.side_to_move] * taper!(phase, POS.state.mg, POS.state.eg)
    }
}

pub fn calc() -> (i16, i16, i16) {
    let mut mg = 0;
    let mut eg = 0;
    let mut p = 0;
    let mut pcs;
    let mut count;
    let mut idx;
    for (i, side) in unsafe{POS.sides.iter().enumerate()} {
        let factor = SIDE_FACTOR[i];
        for j in 0..6 {
            pcs = unsafe{POS.pieces[j]} & side;
            count = pcs.count_ones() as i16;
            p += PHASE_VALS[j] * count;
            while pcs > 0 {
                idx = lsb!(pcs) as usize;
                let white = (i == 0) as usize * 56;
                mg += factor * PST_MG[j][idx ^ white];
                eg += factor * PST_EG[j][idx ^ white];
                pop!(pcs);
            }
        }
    }
    (p, mg, eg)
}

pub fn is_in_check() -> bool {
    unsafe {
    let king_idx = lsb!(POS.pieces[KING] & POS.sides[POS.side_to_move]) as usize;
    is_square_attacked(king_idx, POS.side_to_move, POS.sides[0] | POS.sides[1])
    }
}