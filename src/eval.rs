use super::consts::*;
use super::position::{POS, NULLS, is_square_attacked};
use super::{lsb, pop};

#[inline(always)]
pub fn eval() -> i16 {
    unsafe {
    let phase = std::cmp::min(POS.state.phase as i32, TPHASE);
    let passers = passers();
    let mg = POS.state.mg + passers * PASSERS_MG;
    let mut eg = POS.state.eg + passers * PASSERS_EG;
    if eg != 0 {eg += mop_up((eg < 0) as usize)}
    SIDE_FACTOR[POS.side_to_move] * ((phase * mg as i32 + (TPHASE - phase) * eg as i32) / TPHASE) as i16
    }
}

#[inline(always)]
pub fn lazy_eval() -> i16 {
    unsafe {
    let phase = std::cmp::min(POS.state.phase as i32, TPHASE);
    SIDE_FACTOR[POS.side_to_move] * ((phase * POS.state.mg as i32 + (TPHASE - phase) * POS.state.eg as i32) / TPHASE) as i16
    }
}

pub fn calc() -> (i16, i16, i16) {
    let mut res = (0,0,0);
    for (i, side) in unsafe{POS.sides.iter().enumerate()} {
        let factor = SIDE_FACTOR[i];
        for j in 0..6 {
            let mut pcs = unsafe{POS.pieces[j]} & side;
            let count = pcs.count_ones() as i16;
            res.0 += PHASE_VALS[j] * count;
            while pcs > 0 {
                let idx = lsb!(pcs) as usize;
                let white = (i == 0) as usize * 56;
                res.1 += factor * PST_MG[j][idx ^ white];
                res.2 += factor * PST_EG[j][idx ^ white];
                pop!(pcs);
            }
        }
    }
    res
}

pub fn is_in_check() -> bool {
    unsafe {
    let king_idx = lsb!(POS.pieces[KING] & POS.sides[POS.side_to_move]) as usize;
    is_square_attacked(king_idx, POS.side_to_move, POS.sides[0] | POS.sides[1])
    }
}

pub fn is_draw_by_repetition(num: u8) -> bool {
    unsafe {
    let l = POS.stack.len();
    if l < 6 || NULLS > 0 { return false }
    let to = l - 1;
    let mut from = l.wrapping_sub(POS.state.halfmove_clock as usize);
    if from > 1024 { from = 0 }
    let mut repetitions_count = 1;
    for i in (from..to).rev().step_by(2) {
        if POS.stack[i].state.zobrist == POS.state.zobrist {
            repetitions_count += 1;
            if repetitions_count >= num { return true }
        }
    }
    false
    } 
}

#[inline(always)]
pub fn is_draw_by_50() -> bool {
    unsafe{NULLS > 0 && POS.state.halfmove_clock >= 100}
}

const SQ1: u64 = 0x55AA55AA55AA55AA;
const SQ2: u64 = 0xAA55AA55AA55AA55;
pub fn is_draw_by_material() -> bool {
    unsafe {
    let pawns = POS.pieces[PAWN];
    if pawns == 0 && POS.state.phase <= 2 {
        if POS.state.phase == 2 {
            let bishops = POS.pieces[BISHOP];
            if bishops & POS.sides[0] != bishops && bishops & POS.sides[1] != bishops && (bishops & SQ1 == bishops || bishops & SQ2 == bishops) {
                return true
            }
            return false
        }
        return true
    }
    false
    }
}

const NOT_A: u64 = 0xfefefefefefefefe;
const NOT_H: u64 = 0x7f7f7f7f7f7f7f7f;

unsafe fn passers() -> i16 {
    let wp = POS.pieces[PAWN] & POS.sides[WHITE];
    let bp = POS.pieces[PAWN] & POS.sides[BLACK];
    let mut fspans = bspans(bp);
    fspans |= (fspans & NOT_H) >> 1 | (fspans & NOT_A) << 1;
    let passers = (wp & !fspans).count_ones() as i16;
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
    let wk = lsb!(POS.pieces[KING] & POS.sides[winning_side]) as usize;
    let lk = lsb!(POS.pieces[KING] & POS.sides[winning_side ^ 1]) as usize;
    SIDE_FACTOR[winning_side] * MD[lk][wk]
}