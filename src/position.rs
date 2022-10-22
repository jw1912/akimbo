use super::{lsb, consts::*, movegen::{bishop_attacks, rook_attacks}};
use std::ptr;

macro_rules! bit {($x:expr) => {1 << $x}}
macro_rules! toggle {
    ($side:expr, $pc:expr, $bit:expr) => {
        POS.pieces[$pc] ^= $bit;
        POS.sides[$side] ^= $bit;
    };
}

// The position is stored as global state
pub static mut POS: Position = Position::new();

// BOARD
#[derive(Clone, Copy)]
pub struct Position {
    pub pieces: [u64; 6],
    pub sides: [u64; 2],
    pub squares: [u8; 64],
    pub side_to_move: usize,
    pub state: GameState,
    pub fullmove_counter: u16,
}
impl Position {
    const fn new() -> Position {
        Position { pieces: [0;6], sides: [0;2], squares: [0; 64], side_to_move: 0, state: GameState { en_passant_sq: 0, halfmove_clock: 0, castle_rights: 0 }, fullmove_counter: 0 }
    }
}

#[derive(Clone, Copy, Default)]
pub struct GameState {
    pub en_passant_sq: u16,
    pub halfmove_clock: u8,
    pub castle_rights: u8,
}
// MOVES
#[derive(Clone, Copy)]
pub struct MoveState {
    pub state: GameState,
    pub m: u16,
    pub moved_pc: u8,
    pub captured_pc: u8,
    pub invalid: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct MoveList {
    pub list: [u16; 256],
    pub len: usize,
}
impl Default for MoveList {
    fn default() -> Self {
        Self {
            list: unsafe {
                #[allow(clippy::uninit_assumed_init)]
                std::mem::MaybeUninit::uninit().assume_init()
            },
            len: 0,
        } 
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
        unsafe {
            ptr::swap(ptr.add(i), ptr.add(j));
        }
    }
}

// FEN
const FILES: [char; 8] = ['a','b','c','d','e','f','g','h'];
const PIECES: [char; 12] = ['P','N','B','R','Q','K','p','n','b','r','q','k'];
pub fn parse_fen(s: &str) {
    unsafe {
    let vec: Vec<&str> = s.split_whitespace().collect();
    POS.pieces = [0;6];
    POS.squares = [EMPTY as u8; 64];
    POS.sides = [0; 2];
    let mut idx: usize = 63;
    let rows: Vec<&str> = vec[0].split('/').collect();
    for row in rows {
        for ch in row.chars().rev() {
            if ch == '/' { continue }
            if !ch.is_numeric() {
                let idx2 = PIECES.iter().position(|&element| element == ch).unwrap_or(6);
                let (col, pc) = ((idx2 > 5) as usize, idx2 - 6 * ((idx2 > 5) as usize));
                toggle!(col, pc, 1 << idx);
                POS.squares[idx] = pc as u8;
                idx -= (idx > 0) as usize;
            } else {
                let len = ch.to_string().parse::<usize>().unwrap_or(8);
                idx -= (idx >= len) as usize * len;
            }
        }
    }
    POS.side_to_move = match vec[1] { "w" => WHITE, "b" => BLACK, _ => panic!("") };
    let mut castle_rights = CastleRights::NONE;
    for ch in vec[2].chars() {
        castle_rights |= match ch {'Q' => CastleRights::WHITE_QS, 'K' => CastleRights::WHITE_KS, 'q' => CastleRights::BLACK_QS, 'k' => CastleRights::BLACK_KS, _ => 0,};
    }
    let en_passant_sq = if vec[3] == "-" {0} else {
        let arr: Vec<char> = vec[3].chars().collect();
        let rank: u16 = arr[1].to_string().parse::<u16>().unwrap_or(0) - 1;
        let file = FILES.iter().position(|&c| c == arr[0]).unwrap_or(0);
        8 * rank + file as u16
    };
    let halfmove_clock = vec[4].parse::<u8>().unwrap_or(0);
    POS.state = GameState {en_passant_sq, halfmove_clock, castle_rights};
    POS.fullmove_counter = vec[5].parse::<u16>().unwrap_or(1);
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

pub fn do_move(m: u16) -> MoveState {
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
    // initial updates
    let mut ctx = MoveState { state: POS.state, m, moved_pc, captured_pc , invalid: true};
    toggle!(POS.side_to_move, moved_pc as usize, f | t);
    POS.squares[from] = EMPTY as u8;
    POS.squares[to] = moved_pc;
    POS.state.en_passant_sq = 0;
    // captures
    if captured_pc != EMPTY as u8 {
        let cpc = captured_pc as usize;
        toggle!(opp, cpc, t);
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
                POS.squares[pwn] = EMPTY as u8;
            } else if flag == MoveFlags::DBL_PUSH {
                POS.state.en_passant_sq = match POS.side_to_move {WHITE => to - 8, BLACK => to + 8, _ => panic!("")} as u16;
            } else if flag >= MoveFlags::KNIGHT_PROMO {
                let promo_pc = ((flag >> 12) & 3) + 1; 
                let ppc = promo_pc as usize;
                POS.pieces[moved_pc as usize] ^= t;
                POS.pieces[ppc] ^= t;
                POS.squares[to] = promo_pc as u8;
            }
        } 
        KING => {
            POS.state.castle_rights &= CASTLE_RIGHTS[from];
            if flag == MoveFlags::KS_CASTLE || flag == MoveFlags::QS_CASTLE {
                let (c, idx1, idx2) = CASTLE_MOVES[POS.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                POS.squares.swap(idx1, idx2);
                toggle!(POS.side_to_move, ROOK, c);
            }
        } 
        ROOK => {
            POS.state.castle_rights &= CASTLE_RIGHTS[from];
        }
        _ => {}
    }
    // final updates
    POS.fullmove_counter += (POS.side_to_move == BLACK) as u16;
    POS.state.halfmove_clock = (moved_pc > PAWN as u8 && flag != MoveFlags::CAPTURE) as u8 * (POS.state.halfmove_clock + 1);
    POS.side_to_move ^= 1;
    // is legal?
    let king_idx = lsb!(POS.pieces[KING] & POS.sides[opp ^ 1]) as usize;
    let invalid = is_square_attacked(king_idx, opp ^ 1, POS.sides[0] | POS.sides[1]);
    if invalid { undo_move(ctx) }
    ctx.invalid = invalid;
    ctx
    }
}

pub fn undo_move(state: MoveState) {
    unsafe {
    let opp = POS.side_to_move;
    POS.side_to_move ^= 1;
    // move data
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