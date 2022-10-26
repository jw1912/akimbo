use super::{lsb, pop, consts::*, movegen::{bishop_attacks, rook_attacks}, hash::zobrist::ZVALS};
use std::ptr;

// The position is stored as global state
pub static mut POS: Position = Position { 
    pieces: [0; 6], sides: [0; 2], squares: [0; 64], side_to_move: 0, 
    state: GameState { zobrist: 0, phase: 0, mg: 0, eg: 0, en_passant_sq: 0, halfmove_clock: 0, castle_rights: 0 }, 
    fullmove_counter: 0, stack: Vec::new()
};
// count of how many nulls made to reach position
pub static mut NULLS: u8 = 0;

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

// movelist struct stores moves and scores
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

// MAKING MOVES
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

pub fn do_move(m: u16) -> bool {
    unsafe {
    let opp: usize = POS.side_to_move ^ 1;
    // move data
    let from: usize = ((m >> 6) & 63) as usize;
    let to: usize = (m & 63) as usize;
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
    // move data
    let state: MoveState = POS.stack.pop().unwrap();
    let m: u16 = state.m;
    let moved_pc: u8 = state.moved_pc;
    let captured_pc: u8 = state.captured_pc;
    let from: usize = ((m >> 6) & 63) as usize;
    let to: usize = (m & 63) as usize;
    let f: u64 = bit!(from);
    let t: u64 = bit!(to);
    let flag: u16 = m & MoveFlags::ALL;
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
    POS.fullmove_counter -= (POS.side_to_move == BLACK) as u16;
    }
}

// NULL MOVES
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