use super::{consts::*, position::{POS, MoveList, is_in_check, is_square_attacked}};
use std::hint::unreachable_unchecked;

/// Forward bitscan.
#[macro_export]
macro_rules! lsb {($x:expr) => {$x.trailing_zeros() as u16}}

/// Reverse bitscan.
macro_rules! msb {($x:expr) => {63 ^ $x.leading_zeros() as u16}}

/// Popping the least significant bit from a number.
#[macro_export]
macro_rules! pop {($x:expr) => {$x &= $x - 1}}

macro_rules! pop_lsb {($idx:expr, $x:expr) => {$idx = $x.trailing_zeros() as u16; $x &= $x - 1}}

pub const ALL: bool = true;
pub const CAPTURES: bool = false;

#[inline(always)]
fn encode_moves(move_list: &mut MoveList, mut attacks: u64, from: u16, flag: u16) {
    let f: u16 = from << 6;
    let mut aidx: u16;
    while attacks > 0 {
        pop_lsb!(aidx, attacks);
        move_list.push(flag | f | aidx);
    }
}

pub fn gen_moves<const QUIETS: bool>(move_list: &mut MoveList) {
    unsafe {
    let occupied: u64 = POS.sides[0] | POS.sides[1];
    let friendly: u64 = POS.sides[POS.side_to_move];
    let pawns: u64 = POS.pieces[PAWN] & POS.sides[POS.side_to_move];
    if QUIETS {
        match POS.side_to_move {
            0 => pawn_pushes::<WHITE>(move_list, occupied, pawns),
            1 => pawn_pushes::<BLACK>(move_list, occupied, pawns),
            _ => unreachable_unchecked(),
        }
        if POS.state.castle_rights & CastleRights::SIDES[POS.side_to_move] > 0 && !is_in_check() {castles(move_list, occupied)}
    }
    pawn_captures(move_list, pawns, POS.sides[POS.side_to_move ^ 1]);
    if POS.state.en_passant_sq > 0 {en_passants(move_list, pawns, POS.state.en_passant_sq)}
    piece_moves::<KNIGHT, QUIETS>(move_list, occupied, friendly);
    piece_moves::<BISHOP, QUIETS>(move_list, occupied, friendly);
    piece_moves::<ROOK  , QUIETS>(move_list, occupied, friendly);
    piece_moves::<QUEEN , QUIETS>(move_list, occupied, friendly);
    piece_moves::<KING  , QUIETS>(move_list, occupied, friendly);
    }
}

unsafe fn piece_moves<const PIECE: usize, const QUIETS: bool>(move_list: &mut MoveList, occupied: u64, friendly: u64) {
    let mut from: u16;
    let mut idx: usize;
    let mut attacks: u64;
    let mut attackers: u64 = POS.pieces[PIECE] & friendly;
    while attackers > 0 {
        pop_lsb!(from, attackers);
        idx = from as usize;
        attacks = match PIECE {
            KNIGHT => KNIGHT_ATTACKS[idx],
            ROOK => rook_attacks(idx, occupied),
            BISHOP => bishop_attacks(idx, occupied),
            QUEEN => rook_attacks(idx, occupied) | bishop_attacks(idx, occupied),
            KING => KING_ATTACKS[idx],
            _ => panic!("Not a valid usize in fn piece_moves_general: {}", PIECE),
        };
        encode_moves(move_list, attacks & POS.sides[POS.side_to_move ^ 1], from, MoveFlags::CAPTURE);
        if QUIETS {encode_moves(move_list, attacks & !occupied, from, MoveFlags::QUIET)}
    }
}

#[inline(always)]
unsafe fn pawn_captures(move_list: &mut MoveList, mut attackers: u64, opponents: u64) {
    let mut from: u16;
    let mut attacks: u64;
    let mut cidx: u16;
    let mut f: u16;
    let mut promo_attackers: u64 = attackers & PENRANK[POS.side_to_move];
    attackers &= !PENRANK[POS.side_to_move];
    while attackers > 0 {
        pop_lsb!(from, attackers);
        attacks = PAWN_ATTACKS[POS.side_to_move][from as usize] & opponents;
        encode_moves(move_list, attacks, from, MoveFlags::CAPTURE);
    }
    while promo_attackers > 0 {
        pop_lsb!(from, promo_attackers);
        attacks = PAWN_ATTACKS[POS.side_to_move][from as usize] & opponents;
        while attacks > 0 {
            pop_lsb!(cidx, attacks);
            f = from << 6;
            move_list.push(MoveFlags::QUEEN_PROMO_CAPTURE  | cidx | f);
            move_list.push(MoveFlags::KNIGHT_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::BISHOP_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::ROOK_PROMO_CAPTURE   | cidx | f);
        }
    }
}

#[inline(always)]
unsafe fn en_passants(move_list: &mut MoveList, pawns: u64, sq: u16) {
    let mut attackers: u64 = PAWN_ATTACKS[POS.side_to_move ^ 1][sq as usize] & pawns;
    let mut cidx: u16;
    while attackers > 0 {
        pop_lsb!(cidx, attackers);
        move_list.push( MoveFlags::EN_PASSANT | sq | cidx << 6 );
    }
}

#[inline(always)]
pub fn rook_attacks(idx: usize, occupied: u64) -> u64 {
    let masks: Mask = MASKS[idx];

    // file
    let mut forward: u64 = occupied & masks.file;
    let mut reverse: u64 = forward.swap_bytes();
    forward -= masks.bitmask;
    reverse -= masks.bitmask.swap_bytes();
    forward ^= reverse.swap_bytes();
    forward &= masks.file;

    // rank
    let mut easts: u64 = EAST[idx];
    let mut blockers: u64 = easts & occupied;
    let mut sq: usize = lsb!(blockers | MSB) as usize;
    easts ^= EAST[sq];
    let mut wests: u64 = WEST[idx];
    blockers = wests & occupied;
    sq = msb!(blockers | LSB) as usize;
    wests ^= WEST[sq];

    forward | easts | wests
}

#[inline(always)]
pub fn bishop_attacks(idx: usize, occ: u64) -> u64 {
    let masks: Mask = MASKS[idx];

    // diagonal
    let mut forward: u64 = occ & masks.diag;
    let mut reverse: u64 = forward.swap_bytes();
    forward -= masks.bitmask;
    reverse -= masks.bitmask.swap_bytes();
    forward ^= reverse.swap_bytes();
    forward &= masks.diag;

    // antidiagonal
    let mut forward2: u64 = occ & masks.antidiag;
    let mut reverse2: u64 = forward2.swap_bytes();
    forward2 -= masks.bitmask;
    reverse2 -= masks.bitmask.swap_bytes();
    forward2 ^= reverse2.swap_bytes();
    forward2 &= masks.antidiag;

    forward | forward2
}

#[inline(always)]
fn shift<const SIDE: usize, const AMOUNT: u8>(bb: u64) -> u64 {
    match SIDE {
        WHITE => bb >> AMOUNT,
        BLACK => bb << AMOUNT,
        _ => panic!("Invalid side in fn shift!"),
    }
}

#[inline(always)]
fn idx_shift<const SIDE: usize, const AMOUNT: u16>(idx: u16) -> u16 {
    match SIDE {
        WHITE => idx + AMOUNT,
        BLACK => idx - AMOUNT,
        _ => panic!("Invalid side in fn shift!"),
    }
}

fn pawn_pushes<const SIDE: usize>(move_list: &mut MoveList, occupied: u64, pawns: u64) {
    let empty: u64 = !occupied;
    let mut pushable_pawns: u64 = shift::<SIDE, 8>(empty) & pawns;
    let mut dbl_pushable_pawns: u64 = shift::<SIDE, 8>(shift::<SIDE, 8>(empty & DBLRANK[SIDE]) & empty) & pawns;
    let mut promotable_pawns: u64 = pushable_pawns & PENRANK[SIDE];
    pushable_pawns &= !PENRANK[SIDE];
    let mut idx: u16;
    while pushable_pawns > 0 {
        pop_lsb!(idx, pushable_pawns);
        move_list.push(idx_shift::<SIDE, 8>(idx) | idx << 6);
    }
    while promotable_pawns > 0 {
        pop_lsb!(idx, promotable_pawns);
        let to: u16 = idx_shift::<SIDE, 8>(idx);
        let f: u16 = idx << 6;
        move_list.push(MoveFlags::QUEEN_PROMO  | to | f);
        move_list.push(MoveFlags::KNIGHT_PROMO | to | f);
        move_list.push(MoveFlags::BISHOP_PROMO | to | f);
        move_list.push(MoveFlags::ROOK_PROMO   | to | f);
    }
    while dbl_pushable_pawns > 0 {
        pop_lsb!(idx, dbl_pushable_pawns);
        move_list.push(MoveFlags::DBL_PUSH | idx_shift::<SIDE, 16>(idx) | idx << 6);
    }
}


#[inline(always)]
unsafe fn castles(move_list: &mut MoveList, occupied: u64) {
    match POS.side_to_move {
        WHITE => {
            if POS.state.castle_rights & CastleRights::WHITE_QS > 0 && occupied & (B1C1D1) == 0
                && !is_square_attacked(3, WHITE, occupied) {
                move_list.push(MoveFlags::QS_CASTLE | 2 | 4 << 6)
            }
            if POS.state.castle_rights & CastleRights::WHITE_KS > 0 && occupied & (F1G1) == 0
                && !is_square_attacked(5, WHITE, occupied) {
                move_list.push(MoveFlags::KS_CASTLE | 6 | 4 << 6)
            }
        }
        BLACK => {
            if POS.state.castle_rights & CastleRights::BLACK_QS > 0 && occupied & (B8C8D8) == 0
                && !is_square_attacked(59, BLACK, occupied) {
                move_list.push(MoveFlags::QS_CASTLE | 58 | 60 << 6)
            }
            if POS.state.castle_rights & CastleRights::BLACK_KS > 0 && occupied & (F8G8) == 0
                && !is_square_attacked(61, BLACK, occupied) {
                move_list.push(MoveFlags::KS_CASTLE | 62 | 60 << 6)
            }
        }
        _ => unreachable_unchecked(),
    }
}
