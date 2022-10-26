use super::{lsb, pop, consts::*, movegen::{bishop_attacks, rook_attacks}, hash::zobrist::ZVALS};
use std::ptr;

/// The position is stored as global state.
pub static mut POS: Position = Position { 
    pieces: [0; 6], sides: [0; 2], squares: [0; 64], side_to_move: 0, 
    state: GameState { zobrist: 0, phase: 0, mg: 0, eg: 0, en_passant_sq: 0, halfmove_clock: 0, castle_rights: 0 }, 
    fullmove_counter: 0, stack: Vec::new()
};
/// Count of how many null moves were made during reaching the current position.
pub static mut NULLS: u8 = 0;

/// Removes/adds a piece to the position bitboards.
#[macro_export]
macro_rules! toggle {
    ($side:expr, $pc:expr, $bit:expr) => {
        POS.pieces[$pc] ^= $bit;
        POS.sides[$side] ^= $bit;
    };
}

/// Removes a piece from the incrementalally updated fields.
macro_rules! remove {
    ($from:expr, $side:expr, $pc:expr) => {
        let indx = $from ^ (56 * ($side == 0) as usize);
        POS.state.zobrist ^= ZVALS.pieces[$side][$pc][$from];
        POS.state.mg -= SIDE_FACTOR[$side] * PST_MG[$pc][indx];
        POS.state.eg -= SIDE_FACTOR[$side] * PST_EG[$pc][indx];
    };
}

/// Adds a piece from the incrementalally updated fields.
macro_rules! add {
    ($from:expr, $side:expr, $pc:expr) => {
        let indx = $from ^ (56 * ($side == 0) as usize);
        POS.state.zobrist ^= ZVALS.pieces[$side][$pc][$from];
        POS.state.mg += SIDE_FACTOR[$side] * PST_MG[$pc][indx];
        POS.state.eg += SIDE_FACTOR[$side] * PST_EG[$pc][indx];
    };
}

/// Index of a square -> bitboard with just that square.
macro_rules! bit {($x:expr) => {1 << $x}}

/// Contains all relevant information for the current board state.
pub struct Position {
    /// Array of bitboards, one for each piece type.
    pub pieces: [u64; 6],
    /// Occupancy bitboards for each side.
    pub sides: [u64; 2],
    /// List of the pieces on each square.
    pub squares: [u8; 64],
    /// Side that is about to move.
    pub side_to_move: usize,
    /// Current state that will be pushed to the state stack.
    pub state: GameState,
    /// Number of full moves played to reach the current position.
    pub fullmove_counter: u16,
    /// State stack (history) of the position.
    pub stack: Vec<MoveState>,
}

/// Holds state of the position, to be copied during move-making.
#[derive(Clone, Copy, Default)]
pub struct GameState {
    /// Zobrist hash key for the position.
    pub zobrist: u64,
    /// Current game-phase heuristic.
    pub phase: i16,
    /// Current midgame piece-square table eval.
    pub mg: i16,
    /// Current endgame piece-square table eval.
    pub eg: i16,
    /// Target square for en passant (0 if not available).
    pub en_passant_sq: u16,
    /// Number of half-moves without a capture or pawn push.
    pub halfmove_clock: u8,
    /// Castling rights for each side.
    pub castle_rights: u8,
}

/// Holds all relevant move information that needs to be retrieved on unmake.
#[derive(Clone, Copy)]
pub struct MoveState {
    /// Game state.
    pub state: GameState,
    /// Last move.
    pub m: u16,
    /// Piece last moved.
    pub moved_pc: u8,
    /// Piece last captured.
    pub captured_pc: u8,
}

/// Stack allocated list, to holds moves and move scores.
pub struct MoveList {
    /// List, 256 moves available.
    pub list: [u16; 256],
    /// Length (used capacity) of the list.
    pub len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {list: unsafe {#[allow(clippy::uninit_assumed_init)] std::mem::MaybeUninit::uninit().assume_init()}, len: 0} 
    }
}

impl MoveList {
    /// Pushes an item to the move list.
    #[inline(always)]
    pub fn push(&mut self, m: u16) {
        self.list[self.len] = m;
        self.len += 1;
    }

    /// Swaps two items in the move list.
    #[inline(always)]
    pub fn swap_unchecked(&mut self, i: usize, j: usize) {
        let ptr: *mut u16 = self.list.as_mut_ptr();
        unsafe { ptr::swap(ptr.add(i), ptr.add(j)) }
    }
}

/// Is the given square under attack by the opposing side?
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

/// Is the current side to move in check?
#[inline(always)]
pub fn is_in_check() -> bool {
    unsafe {
    let king_idx: usize = lsb!(POS.pieces[KING] & POS.sides[POS.side_to_move]) as usize;
    is_square_attacked(king_idx, POS.side_to_move, POS.sides[0] | POS.sides[1])
    }
}

/// Makes a given move.
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

/// Undoes the last move played.
pub fn undo_move() {
    unsafe {
    let opp: usize = POS.side_to_move;
    POS.side_to_move ^= 1;

    // restore state
    let state: MoveState = POS.stack.pop().unwrap();

    // move data
    let moved_pc: u8 = state.moved_pc;
    let captured_pc: u8 = state.captured_pc;
    let from: usize = ((state.m >> 6) & 63) as usize;
    let to: usize = (state.m & 63) as usize;
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

    // fnal updates
    POS.fullmove_counter -= (POS.side_to_move == BLACK) as u16;
    }
}

/// Makes a null move.
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

/// Undoes a null move.
pub fn undo_null((enp, hash): (u16, u64)) {
    unsafe {
    NULLS -= 1;
    POS.state.zobrist = hash;
    POS.state.en_passant_sq = enp;
    POS.side_to_move ^= 1;
    }
}

/// Checks for an n-fold repetition by going through the state stack and
/// comparing zobrist keys.
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

/// Has the position reached a draw by the fifty-move rule.
#[inline(always)]
pub fn is_draw_by_50() -> bool {
    unsafe{NULLS > 0 && POS.state.halfmove_clock >= 100}
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