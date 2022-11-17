use crate::position::is_in_check;

use super::{consts::*, position::{POS, is_square_attacked, MoveList}};

/// Forward bitscan.
#[macro_export]
macro_rules! lsb {($x:expr) => {$x.trailing_zeros() as u16}}

/// Reverse bitscan.
macro_rules! msb {($x:expr) => {63 ^ $x.leading_zeros() as u16}}

/// Popping the least significant bit from a number.
#[macro_export]
macro_rules! pop {($x:expr) => {$x &= $x - 1}}

/// Generate all moves.
pub const ALL: u8 = 0;
/// Generate captures only.
pub const CAPTURES: u8 = 1;
/// Generate quiet moves only.
pub const QUIETS: u8 = 2;

/// Encodes a set of attacks with a given move flag and from-square.
#[inline(always)]
fn encode_moves(move_list: &mut MoveList, mut attacks: u64, from: u16, flag: u16) {
    let f: u16 = from << 6;
    let mut aidx: u16;
    while attacks > 0 {
        aidx = lsb!(attacks);
        move_list.push(flag | f | aidx);
        pop!(attacks)
    }
}

/// Generates all moves of a given type in the current position.
pub fn gen_moves<const U: u8>(move_list: &mut MoveList) {
    unsafe {
    let occupied: u64 = POS.sides[0] | POS.sides[1];
    let friendly: u64 = POS.sides[POS.side_to_move];
    let pawns: u64 = POS.pieces[PAWN] & POS.sides[POS.side_to_move];
    if U != CAPTURES {
        match POS.side_to_move {
            0 => pawn_pushes::<{ WHITE }>(move_list, occupied, pawns),
            1 => pawn_pushes::<{ BLACK }>(move_list, occupied, pawns),
            _ => panic!("Invalid side to move!"),
        }
        if POS.state.castle_rights & CastleRights::SIDES[POS.side_to_move] > 0 {
            castles(move_list, occupied);
        }
    }
    if U != QUIETS {
        let opps: u64 = POS.sides[POS.side_to_move ^ 1];
        pawn_captures(move_list, pawns, opps);
        if POS.state.en_passant_sq > 0 {en_passants(move_list, pawns, POS.state.en_passant_sq)}
    }
    piece_moves::<{ KNIGHT }, U>(move_list, occupied, friendly);
    piece_moves::<{ BISHOP }, U>(move_list, occupied, friendly);
    piece_moves::<{ ROOK   }, U>(move_list, occupied, friendly);
    piece_moves::<{ QUEEN  }, U>(move_list, occupied, friendly);
    piece_moves::<{ KING   }, U>(move_list, occupied, friendly);
    }
}

unsafe fn piece_moves<const PIECE: usize, const U: u8>(move_list: &mut MoveList, occupied: u64, friendly: u64) {
    let mut from: u16;
    let mut idx: usize;
    let mut attacks: u64;
    let mut attackers: u64 = POS.pieces[PIECE] & friendly;
    while attackers > 0 {
        from = lsb!(attackers);
        idx = from as usize;
        attacks = match PIECE {
            KNIGHT => KNIGHT_ATTACKS[idx],
            ROOK => rook_attacks(idx, occupied),
            BISHOP => bishop_attacks(idx, occupied),
            QUEEN => rook_attacks(idx, occupied) | bishop_attacks(idx, occupied),
            KING => KING_ATTACKS[idx],
            _ => panic!("Not a valid usize in fn piece_moves_general: {}", PIECE),
        };
        if U != CAPTURES {encode_moves(move_list, attacks & !occupied, from, MoveFlags::QUIET)}
        if U != QUIETS {encode_moves(move_list, attacks & POS.sides[POS.side_to_move ^ 1], from, MoveFlags::CAPTURE)}
        pop!(attackers)
    }
}

#[inline(always)]
unsafe fn pawn_captures(move_list: &mut MoveList, mut attackers: u64, opponents: u64) {
    let mut from: u16;
    let mut attacks: u64;
    let mut promo_attackers: u64 = attackers & PENRANK[POS.side_to_move];
    attackers &= !PENRANK[POS.side_to_move];
    while attackers > 0 {
        from = lsb!(attackers);
        attacks = PAWN_ATTACKS[POS.side_to_move][from as usize] & opponents;
        encode_moves(move_list, attacks, from, MoveFlags::CAPTURE);
        pop!(attackers)
    }
    let mut cidx: u16;
    while promo_attackers > 0 {
        from = lsb!(promo_attackers);
        attacks = PAWN_ATTACKS[POS.side_to_move][from as usize] & opponents;
        while attacks > 0 {
            cidx = lsb!(attacks);
            let f: u16 = from << 6;
            move_list.push(MoveFlags::KNIGHT_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::BISHOP_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::ROOK_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::QUEEN_PROMO_CAPTURE | cidx | f);
            pop!(attacks)
        }
        pop!(promo_attackers)
    }
}

#[inline(always)]
unsafe fn en_passants(move_list: &mut MoveList, pawns: u64, sq: u16) {
    let mut attackers: u64 = PAWN_ATTACKS[POS.side_to_move ^ 1][sq as usize] & pawns;
    while attackers > 0 {
        let cidx: u16 = lsb!(attackers);
        move_list.push( MoveFlags::EN_PASSANT | sq | cidx << 6 );
        pop!(attackers)
    }
}

/// Calculates rook attacks from a given square and occupancy.
#[inline(always)]
pub fn rook_attacks(idx: usize, occupied: u64) -> u64 {
    let masks: Rmask = RMASKS[idx];

    // forward moves
    let mut forward: u64 = occupied & masks.file;
    let mut reverse: u64 = forward.swap_bytes();
    forward -= masks.bitmask;
    reverse -= masks.bitmask.swap_bytes();
    forward ^= reverse.swap_bytes();
    forward &= masks.file;

    // backwards moves
    let mut easts: u64 = EAST[idx];
    let mut blockers: u64 = easts & occupied;
    let mut sq = lsb!(blockers | MSB) as usize;
    easts ^= EAST[sq];
    let mut wests: u64 = masks.wests;
    blockers = wests & occupied;
    sq = msb!(blockers | LSB) as usize;
    wests ^= WEST[sq];

    forward | easts | wests
}

/// Calculates bishop attacks from a given square and occupancy.
#[inline(always)]
pub fn bishop_attacks(idx: usize, occ: u64) -> u64 {
    let masks: Mask = MASKS[idx];

    // forward moves
    let mut forward: u64 = occ & masks.diag;
    let mut reverse: u64 = forward.swap_bytes();
    forward -= masks.bitmask;
    reverse -= masks.bitmask.swap_bytes();
    forward ^= reverse.swap_bytes();
    forward &= masks.diag;

    // backward moves
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
        idx = lsb!(pushable_pawns);
        pop!(pushable_pawns);
        move_list.push(idx_shift::<SIDE, 8>(idx) | idx << 6);
    }
    while promotable_pawns > 0 {
        idx = lsb!(promotable_pawns);
        pop!(promotable_pawns);
        let to: u16 = idx_shift::<SIDE, 8>(idx);
        let f: u16 = idx << 6;
        move_list.push(MoveFlags::KNIGHT_PROMO | to | f);
        move_list.push(MoveFlags::BISHOP_PROMO | to | f);
        move_list.push(MoveFlags::ROOK_PROMO | to | f);
        move_list.push(MoveFlags::QUEEN_PROMO | to | f);
    }
    while dbl_pushable_pawns > 0 {
        idx = lsb!(dbl_pushable_pawns);
        pop!(dbl_pushable_pawns);
        move_list.push(MoveFlags::DBL_PUSH | idx_shift::<SIDE, 16>(idx) | idx << 6);
    }
}


#[inline(always)]
unsafe fn castles(move_list: &mut MoveList, occupied: u64) {
    if is_in_check() { return }
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
        _ => panic!("Invalid side for castling!"),
    }
}
