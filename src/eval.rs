use super::consts::*;
use super::position::POS;
use super::{lsb, pop};

/// Returns a piece-square table only evaluation of the current position.
#[inline(always)]
pub fn lazy_eval() -> i16 {
    unsafe {
    let phase: i32 = std::cmp::min(POS.state.phase as i32, TPHASE);
    SIDE_FACTOR[POS.side_to_move] * ((phase * POS.state.mg as i32 + (TPHASE - phase) * POS.state.eg as i32) / TPHASE) as i16
    }
}

/// Calculates the midgame and endgame piece-square table evaluations and the game 
/// phase of the current position from scratch.
pub fn calc() -> (i16, i16, i16) {
    let mut res: (i16, i16, i16) = (0,0,0);
    for (i, side) in unsafe{POS.sides.iter().enumerate()} {
        let factor = SIDE_FACTOR[i];
        for j in 0..6 {
            let mut pcs: u64 = unsafe{POS.pieces[j]} & side;
            let count: i16 = pcs.count_ones() as i16;
            res.0 += PHASE_VALS[j] * count;
            while pcs > 0 {
                let idx: usize = lsb!(pcs) as usize;
                let white: usize = (i == 0) as usize * 56;
                res.1 += factor * PST_MG[j][idx ^ white];
                res.2 += factor * PST_EG[j][idx ^ white];
                pop!(pcs);
            }
        }
    }
    res
}
