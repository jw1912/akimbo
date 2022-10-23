use super::{lsb, pop, consts::*, movegen::{bishop_attacks, rook_attacks}, hash::zobrist::ZVALS};
use std::ptr;

// The position is stored as global state
pub static mut POS: Position = Position::new();

// MACROS
macro_rules! bit {($x:expr) => {1 << $x}}
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

// STRUCTS
pub struct Position {
    pub pieces: [u64; 6],
    pub sides: [u64; 2],
    pub squares: [u8; 64],
    pub side_to_move: usize,
    pub state: GameState,
    pub fullmove_counter: u16,
    pub stack: Vec<MoveState>,
}
impl Position {
    const fn new() -> Position {
        Position { pieces: [0;6], sides: [0;2], squares: [0; 64], side_to_move: 0, state: GameState { zobrist: 0, phase: 0, mg: 0, eg: 0, en_passant_sq: 0, halfmove_clock: 0, castle_rights: 0 }, fullmove_counter: 0, stack: Vec::new() }
    }
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
    pub state: GameState,
    pub m: u16,
    pub moved_pc: u8,
    pub captured_pc: u8,
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
        let ptr = self.list.as_mut_ptr();
        unsafe { ptr::swap(ptr.add(i), ptr.add(j)) }
    }
}

// MAKING MOVES
#[inline(always)]
pub fn is_square_attacked(idx: usize, side: usize, occ: u64) -> bool {
    unsafe {
    let other = side ^ 1;
    let s = POS.sides[other];
    let opp_queen = POS.pieces[QUEEN] & s;
    (KNIGHT_ATTACKS[idx] & POS.pieces[KNIGHT] & s > 0)
    || (KING_ATTACKS[idx] & POS.pieces[KING] & s > 0)
    || (PAWN_ATTACKS[side][idx] & POS.pieces[PAWN] & s > 0)
    || (rook_attacks(idx, occ) & (POS.pieces[ROOK] & s | opp_queen) > 0)
    || (bishop_attacks(idx, occ) & (POS.pieces[BISHOP] & s | opp_queen) > 0)
    }
}

pub fn do_move(m: u16) -> bool {
    unsafe {
    let opp = POS.side_to_move ^ 1;
    // move data
    let from = ((m >> 6) & 63) as usize;
    let to = (m & 63) as usize;
    let f = bit!(from);
    let t = bit!(to);
    let moved_pc = POS.squares[from];
    let captured_pc = POS.squares[to];
    let flag = m & MoveFlags::ALL;
    let rights = POS.state.castle_rights;
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
        let cpc = captured_pc as usize;
        toggle!(opp, cpc, t);
        remove!(to, opp, cpc);
        POS.state.phase -= PHASE_VALS[cpc];
        if captured_pc == ROOK as u8 {
            POS.state.castle_rights &= CASTLE_RIGHTS[to];
        }
    }
    match moved_pc as usize { 
        PAWN =>  {
            if flag == MoveFlags::EN_PASSANT {
                let pwn = match opp { WHITE => to + 8, BLACK => to - 8, _ => panic!() };
                let p = bit!(pwn);
                toggle!(opp, PAWN, p);
                remove!(pwn, opp, PAWN);
                POS.squares[pwn] = EMPTY as u8;
            } else if flag == MoveFlags::DBL_PUSH {
                POS.state.en_passant_sq = match POS.side_to_move {WHITE => to - 8, BLACK => to + 8, _ => panic!("")} as u16;
                POS.state.zobrist ^= ZVALS.en_passant[to & 7];
            } else if flag >= MoveFlags::KNIGHT_PROMO {
                let promo_pc = ((flag >> 12) & 3) + 1; 
                let ppc = promo_pc as usize;
                POS.pieces[moved_pc as usize] ^= t;
                POS.pieces[ppc] ^= t;
                POS.squares[to] = promo_pc as u8;
                POS.state.phase += PHASE_VALS[ppc];
                remove!(to, POS.side_to_move, moved_pc as usize);
                add!(to, POS.side_to_move, ppc);
            }
        } 
        KING => {
            POS.state.castle_rights &= CASTLE_RIGHTS[from];
            if flag == MoveFlags::KS_CASTLE || flag == MoveFlags::QS_CASTLE {
                let (c, idx1, idx2) = CASTLE_MOVES[POS.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                POS.squares.swap(idx1, idx2);
                toggle!(POS.side_to_move, ROOK, c);
                remove!(idx1, POS.side_to_move, ROOK);
                add!(idx2, POS.side_to_move, ROOK);
            }
        } 
        ROOK => {
            POS.state.castle_rights &= CASTLE_RIGHTS[from];
        }
        _ => {}
    }
    // castle hashes
    let mut changed_castle = rights & !POS.state.castle_rights;
    while changed_castle > 0 {
        let ls1b = changed_castle & changed_castle.wrapping_neg();
        POS.state.zobrist ^= ZVALS.castle_hash(rights, ls1b);
        pop!(changed_castle)
    }
    // final updates
    POS.fullmove_counter += (POS.side_to_move == BLACK) as u16;
    POS.state.halfmove_clock = (moved_pc > PAWN as u8 && flag != MoveFlags::CAPTURE) as u8 * (POS.state.halfmove_clock + 1);
    POS.side_to_move ^= 1;
    // is legal?
    let king_idx = lsb!(POS.pieces[KING] & POS.sides[opp ^ 1]) as usize;
    let invalid = is_square_attacked(king_idx, opp ^ 1, POS.sides[0] | POS.sides[1]);
    if invalid { undo_move() }
    invalid
    }
}

pub fn undo_move() {
    unsafe {
    let opp = POS.side_to_move;
    POS.side_to_move ^= 1;
    // move data
    let state = POS.stack.pop().unwrap();
    let m = state.m;
    let moved_pc = state.moved_pc;
    let captured_pc = state.captured_pc;
    let from = ((m >> 6) & 63) as usize;
    let to = (m & 63) as usize;
    let f = bit!(from);
    let t = bit!(to);
    let flag = m & MoveFlags::ALL;
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
    match moved_pc as usize { 
        PAWN =>  {
            if flag == MoveFlags::EN_PASSANT {
                let pwn = match opp { WHITE => to + 8, BLACK => to - 8, _ => panic!() };
                let p = bit!(pwn);
                toggle!(opp, PAWN, p);
                POS.squares[pwn] = PAWN as u8;
            } else if flag >= MoveFlags::KNIGHT_PROMO {
                let promo_pc = ((flag >> 12) & 3) + 1; 
                POS.pieces[moved_pc as usize] ^= t;
                POS.pieces[promo_pc as usize] ^= t;
            }
        } 
        KING => {
            if flag == MoveFlags::KS_CASTLE || flag == MoveFlags::QS_CASTLE {
                let (c, idx1, idx2) = CASTLE_MOVES[POS.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                POS.squares.swap(idx1, idx2);
                toggle!(POS.side_to_move, ROOK, c);
            }
        } 
        _ => {}
    }
    POS.fullmove_counter -= (POS.side_to_move == BLACK) as u16;
    }
}