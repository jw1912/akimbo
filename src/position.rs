use super::{lsb, pop, consts::*, movegen::{bishop_attacks, rook_attacks}, zobrist::ZVALS};
use std::ptr;

/// The position is stored as global state.
pub static mut POS: Position = Position {
    pieces: [0; 6], sides: [0; 2], squares: [EMPTY as u8; 64], side_to_move: 0,
    state: GameState { zobrist: 0, phase: 0, mg: 0, eg: 0, en_passant_sq: 0, halfmove_clock: 0, castle_rights: 0 },
    fullmove_counter: 0, stack: Vec::new()
};

/// Count of how many null moves were made during reaching the current position.
pub static mut NULLS: u8 = 0;

#[macro_export]
macro_rules! toggle {
    ($side:expr, $pc:expr, $bit:expr) => {
        POS.pieces[$pc] ^= $bit;
        POS.sides[$side] ^= $bit;
    };
}

macro_rules! remove {
    ($from:expr, $side:expr, $pc:expr) => {
        let indx = $from ^ (56 * ($side == 0) as usize);
        POS.state.zobrist ^= ZVALS.pieces[$side][$pc][$from];
        POS.state.mg -= SIDE_FACTOR[$side] * PST_MG[$pc][indx];
        POS.state.eg -= SIDE_FACTOR[$side] * PST_EG[$pc][indx];
    };
}

macro_rules! add {
    ($from:expr, $side:expr, $pc:expr) => {
        let indx = $from ^ (56 * ($side == 0) as usize);
        POS.state.zobrist ^= ZVALS.pieces[$side][$pc][$from];
        POS.state.mg += SIDE_FACTOR[$side] * PST_MG[$pc][indx];
        POS.state.eg += SIDE_FACTOR[$side] * PST_EG[$pc][indx];
    };
}

#[macro_export]
macro_rules! from {($m:expr) => {(($m >> 6) & 63) as usize}}

#[macro_export]
macro_rules! to {($m:expr) => {($m & 63) as usize}}

#[macro_export]
macro_rules! bit {($x:expr) => {1 << $x}}

pub struct Position {
    pub pieces: [u64; 6],
    pub sides: [u64; 2],
    pub squares: [u8; 64],
    pub side_to_move: usize,
    pub state: GameState,
    pub fullmove_counter: u16,
    pub stack: Vec<MoveState>,
}

#[derive(Clone, Copy, Default)]
pub struct GameState {
    pub zobrist: u64,
    pub phase: i16,
    pub mg: i16,
    pub eg: i16,
    pub en_passant_sq: u16,
    pub halfmove_clock: u8,
    pub castle_rights: u8,
}

#[derive(Clone, Copy)]
pub struct MoveState {
    state: GameState,
    m: u16,
    moved_pc: u8,
    captured_pc: u8,
}

pub struct MoveList {
    pub list: [u16; 256],
    pub len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {list: unsafe {#[allow(clippy::uninit_assumed_init)] std::mem::MaybeUninit::uninit().assume_init()}, len: 0} 
    }
}

impl MoveList {
    #[inline(always)]
    pub fn push(&mut self, m: u16) {
        self.list[self.len] = m;
        self.len += 1;
    }

    #[inline(always)]
    pub fn swap_unchecked(&mut self, i: usize, j: usize) {
        let ptr: *mut u16 = self.list.as_mut_ptr();
        unsafe { ptr::swap(ptr.add(i), ptr.add(j)) }
    }
}

#[inline(always)]
pub fn is_square_attacked(idx: usize, side: usize, occ: u64) -> bool {
    unsafe {
    let other: usize = side ^ 1;
    let s: u64 = POS.sides[other];
    let opp_queen: u64 = POS.pieces[QUEEN] & s;
    (KNIGHT_ATTACKS[idx] & POS.pieces[KNIGHT] & s > 0)
    || (KING_ATTACKS[idx] & POS.pieces[KING] & s > 0)
    || (PAWN_ATTACKS[side][idx] & POS.pieces[PAWN] & s > 0)
    || (rook_attacks(idx, occ) & (POS.pieces[ROOK] & s | opp_queen) > 0)
    || (bishop_attacks(idx, occ) & (POS.pieces[BISHOP] & s | opp_queen) > 0)
    }
}

#[inline(always)]
pub fn is_in_check() -> bool {
    unsafe {
    let king_idx: usize = lsb!(POS.pieces[KING] & POS.sides[POS.side_to_move]) as usize;
    is_square_attacked(king_idx, POS.side_to_move, POS.sides[0] | POS.sides[1])
    }
}

pub fn do_move(m: u16) -> bool {
    unsafe {
    let opp: usize = POS.side_to_move ^ 1;

    // move data
    let from: usize = from!(m);
    let to: usize = to!(m);
    let f: u64 = bit!(from);
    let t: u64 = bit!(to);
    let moved_pc: u8 = POS.squares[from];
    let captured_pc: u8 = POS.squares[to];
    let flag: u16 = m & MoveFlags::ALL;
    let rights: u8 = POS.state.castle_rights;

    // initial updates
    POS.stack.push(MoveState { state: POS.state, m, moved_pc, captured_pc});
    toggle!(POS.side_to_move, moved_pc as usize, f | t);
    remove!(from, POS.side_to_move, moved_pc as usize);
    add!(to, POS.side_to_move, moved_pc as usize);
    POS.squares[from] = EMPTY as u8;
    POS.squares[to] = moved_pc;
    if POS.state.en_passant_sq > 0 {POS.state.zobrist ^= ZVALS.en_passant[(POS.state.en_passant_sq & 7) as usize]}
    POS.state.en_passant_sq = 0;
    POS.state.zobrist ^= ZVALS.side;

    // captures
    if captured_pc != EMPTY as u8 {
        let cpc: usize = captured_pc as usize;
        toggle!(opp, cpc, t);
        remove!(to, opp, cpc);
        POS.state.phase -= PHASE_VALS[cpc];
        if captured_pc == ROOK as u8 {
            POS.state.castle_rights &= CASTLE_RIGHTS[to];
        }
    }

    // piece-specific updates
    match moved_pc as usize {
        PAWN =>  {
            if flag == MoveFlags::EN_PASSANT {
                let pwn: usize = match opp { WHITE => to + 8, BLACK => to - 8, _ => panic!() };
                let p: u64 = bit!(pwn);
                toggle!(opp, PAWN, p);
                remove!(pwn, opp, PAWN);
                POS.squares[pwn] = EMPTY as u8;
            } else if flag == MoveFlags::DBL_PUSH {
                POS.state.en_passant_sq = match POS.side_to_move {WHITE => to - 8, BLACK => to + 8, _ => panic!("")} as u16;
                POS.state.zobrist ^= ZVALS.en_passant[to & 7];
            } else if flag >= MoveFlags::KNIGHT_PROMO {
                let ppc: usize = (((flag >> 12) & 3) + 1) as usize;
                POS.pieces[moved_pc as usize] ^= t;
                POS.pieces[ppc] ^= t;
                POS.squares[to] = ppc as u8;
                POS.state.phase += PHASE_VALS[ppc];
                remove!(to, POS.side_to_move, moved_pc as usize);
                add!(to, POS.side_to_move, ppc);
            }
        }
        KING => {
            POS.state.castle_rights &= CASTLE_RIGHTS[from];
            if flag == MoveFlags::KS_CASTLE || flag == MoveFlags::QS_CASTLE {
                let (c, idx1, idx2): (u64, usize, usize) = CASTLE_MOVES[POS.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                POS.squares.swap(idx1, idx2);
                toggle!(POS.side_to_move, ROOK, c);
                remove!(idx1, POS.side_to_move, ROOK);
                add!(idx2, POS.side_to_move, ROOK);
            }
        }
        ROOK => POS.state.castle_rights &= CASTLE_RIGHTS[from],
        _ => {}
    }

    // castle hashes
    let mut changed_castle: u8 = rights & !POS.state.castle_rights;
    while changed_castle > 0 {
        let ls1b: u8 = changed_castle & changed_castle.wrapping_neg();
        POS.state.zobrist ^= ZVALS.castle_hash(rights, ls1b);
        pop!(changed_castle)
    }

    // final updates
    POS.fullmove_counter += (POS.side_to_move == BLACK) as u16;
    POS.state.halfmove_clock = (moved_pc > PAWN as u8 && flag != MoveFlags::CAPTURE) as u8 * (POS.state.halfmove_clock + 1);
    POS.side_to_move ^= 1;

    // is legal?
    let king_idx: usize = lsb!(POS.pieces[KING] & POS.sides[opp ^ 1]) as usize;
    let invalid: bool = is_square_attacked(king_idx, opp ^ 1, POS.sides[0] | POS.sides[1]);
    if invalid { undo_move() }
    invalid
    }
}

pub fn undo_move() {
    unsafe {
    let opp: usize = POS.side_to_move;
    POS.side_to_move ^= 1;

    // restore state
    let state: MoveState = POS.stack.pop().unwrap();

    // move data
    let moved_pc: u8 = state.moved_pc;
    let captured_pc: u8 = state.captured_pc;
    let from: usize = from!(state.m);
    let to: usize = to!(state.m);
    let f: u64 = bit!(from);
    let t: u64 = bit!(to);
    let flag: u16 = state.m & MoveFlags::ALL;

    // initial updates
    POS.state = state.state;
    toggle!(POS.side_to_move, moved_pc as usize, f | t);
    POS.squares[from] = moved_pc;
    POS.squares[to] = captured_pc;

    // captures
    if captured_pc != EMPTY as u8 {
        POS.pieces[captured_pc as usize] ^= t;
        POS.sides[opp] ^= t;
    }

    // piece-specific updates
    match moved_pc as usize {
        PAWN =>  {
            if flag == MoveFlags::EN_PASSANT {
                let pwn: usize = match opp { WHITE => to + 8, BLACK => to - 8, _ => panic!() };
                let p: u64 = bit!(pwn);
                toggle!(opp, PAWN, p);
                POS.squares[pwn] = PAWN as u8;
            } else if flag >= MoveFlags::KNIGHT_PROMO {
                let promo_pc: u16 = ((flag >> 12) & 3) + 1;
                POS.pieces[moved_pc as usize] ^= t;
                POS.pieces[promo_pc as usize] ^= t;
            }
        }
        KING => {
            if flag == MoveFlags::KS_CASTLE || flag == MoveFlags::QS_CASTLE {
                let (c, idx1, idx2): (u64, usize, usize) = CASTLE_MOVES[POS.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                POS.squares.swap(idx1, idx2);
                toggle!(POS.side_to_move, ROOK, c);
            }
        }
        _ => {}
    }

    // final updates
    POS.fullmove_counter -= (POS.side_to_move == BLACK) as u16;
    }
}

pub fn do_null() -> (u16, u64) {
    unsafe {
    NULLS += 1;
    let enp: u16 = POS.state.en_passant_sq;
    let hash: u64 = POS.state.zobrist;
    POS.state.zobrist ^= (enp > 0) as u64 * ZVALS.en_passant[(enp & 7) as usize];
    POS.state.en_passant_sq = 0;
    POS.side_to_move ^= 1;
    POS.state.zobrist ^= ZVALS.side;
    (enp, hash)
    }
}

pub fn undo_null((enp, hash): (u16, u64)) {
    unsafe {
    NULLS -= 1;
    POS.state.zobrist = hash;
    POS.state.en_passant_sq = enp;
    POS.side_to_move ^= 1;
    }
}

pub fn is_draw_by_repetition(num: u8) -> bool {
    unsafe {
    let l: usize = POS.stack.len();
    if l < 6 || NULLS > 0 { return false }
    let to: usize = l - 1;
    let mut from: usize = l.wrapping_sub(POS.state.halfmove_clock as usize);
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
    unsafe{POS.state.halfmove_clock >= 100}
}

/// Is there a FIDE draw by insufficient material?
///  - KvK
///  - KvKN or KvKB
///  - KBvKB and both bishops the same colour
pub fn is_draw_by_material() -> bool {
    unsafe {
    let pawns: u64 = POS.pieces[PAWN];
    if pawns == 0 && POS.state.phase <= 2 {
        if POS.state.phase == 2 {
            let bishops: u64 = POS.pieces[BISHOP];
            return bishops & POS.sides[0] != bishops && bishops & POS.sides[1] != bishops && (bishops & 0x55AA55AA55AA55AA == bishops || bishops & 0xAA55AA55AA55AA55 == bishops)
        }
        return true
    }
    false
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
