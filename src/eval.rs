use super::consts::*;
use super::position::POS;
use super::{lsb, pop};

/// Returns a full static evaluation of the current position.
#[inline(always)]
pub fn static_eval() -> i16 {
    unsafe {
    let phase: i32 = std::cmp::min(POS.state.phase as i32, TPHASE);
    let passers: i16 = passers();
    let mg: i16 = POS.state.mg + passers * PASSERS_MG;
    let mut eg: i16 = POS.state.eg + passers * PASSERS_EG;
    if eg != 0 {eg += mop_up((eg < 0) as usize)}
    SIDE_FACTOR[POS.side_to_move] * ((phase * mg as i32 + (TPHASE - phase) * eg as i32) / TPHASE) as i16
    }
}

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

/// Calculates the net passed pawns from white's perspective.
unsafe fn passers() -> i16 {
    let wp: u64 = POS.pieces[PAWN] & POS.sides[WHITE];
    let bp: u64 = POS.pieces[PAWN] & POS.sides[BLACK];
    let mut fspans: u64 = bspans(bp);
    fspans |= (fspans & NOT_H) >> 1 | (fspans & NOT_A) << 1;
    let passers: i16 = (wp & !fspans).count_ones() as i16;
    fspans = wspans(wp);
    fspans |= (fspans & NOT_H) >> 1 | (fspans & NOT_A) << 1;
    passers - (bp & !fspans).count_ones() as i16
}

#[inline(always)]
fn wspans(mut pwns: u64) -> u64 {
    pwns |= pwns << 8;
    pwns |= pwns << 16;
    pwns |= pwns << 32;
    pwns << 8
}

#[inline(always)]
fn bspans(mut pwns: u64) -> u64 {
    pwns |= pwns >> 8;
    pwns |= pwns >> 16;
    pwns |= pwns >> 32;
    pwns >> 8
}

unsafe fn mop_up(winning_side: usize) -> i16 {
    let wk: usize = lsb!(POS.pieces[KING] & POS.sides[winning_side]) as usize;
    let lk: usize = lsb!(POS.pieces[KING] & POS.sides[winning_side ^ 1]) as usize;
    SIDE_FACTOR[winning_side] * MD[lk][wk]
}