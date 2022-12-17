use super::{consts::*, position::{MoveList, Position}};

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

impl Position {
    pub fn gen_moves<const QUIETS: bool>(&self, move_list: &mut MoveList) {
        let occupied: u64 = self.sides[0] | self.sides[1];
        let friendly: u64 = self.sides[self.side_to_move];
        let opps: u64 = self.sides[self.side_to_move ^ 1];
        let pawns: u64 = self.pieces[PAWN] & self.sides[self.side_to_move];
        if QUIETS {
            if self.side_to_move == WHITE {pawn_pushes::<WHITE>(move_list, occupied, pawns)} else {pawn_pushes::<BLACK>(move_list, occupied, pawns)}
            if self.state.castle_rights & CastleRights::SIDES[self.side_to_move] > 0 && !self.is_in_check() {self.castles(move_list, occupied)}
        }
        pawn_captures(move_list, pawns, opps, self.side_to_move);
        if self.state.en_passant_sq > 0 {en_passants(move_list, pawns, self.state.en_passant_sq, self.side_to_move)}
        piece_moves::<KNIGHT, QUIETS>(move_list, occupied, friendly, opps, self.pieces[KNIGHT]);
        piece_moves::<BISHOP, QUIETS>(move_list, occupied, friendly, opps, self.pieces[BISHOP]);
        piece_moves::<ROOK  , QUIETS>(move_list, occupied, friendly, opps, self.pieces[ROOK]);
        piece_moves::<QUEEN , QUIETS>(move_list, occupied, friendly, opps, self.pieces[QUEEN]);
        piece_moves::<KING  , QUIETS>(move_list, occupied, friendly, opps, self.pieces[KING]);
    }

    #[inline(always)]
    fn castles(&self, move_list: &mut MoveList, occupied: u64) {
        if self.side_to_move == WHITE {
            if self.state.castle_rights & CastleRights::WHITE_QS > 0 && occupied & (B1C1D1) == 0
                && !self.is_square_attacked(3, WHITE, occupied) {
                move_list.push(MoveFlags::QS_CASTLE | 2 | 4 << 6)
            }
            if self.state.castle_rights & CastleRights::WHITE_KS > 0 && occupied & (F1G1) == 0
                && !self.is_square_attacked(5, WHITE, occupied) {
                move_list.push(MoveFlags::KS_CASTLE | 6 | 4 << 6)
            }
        } else {
            if self.state.castle_rights & CastleRights::BLACK_QS > 0 && occupied & (B8C8D8) == 0
                && !self.is_square_attacked(59, BLACK, occupied) {
                move_list.push(MoveFlags::QS_CASTLE | 58 | 60 << 6)
            }
            if self.state.castle_rights & CastleRights::BLACK_KS > 0 && occupied & (F8G8) == 0
                && !self.is_square_attacked(61, BLACK, occupied) {
                move_list.push(MoveFlags::KS_CASTLE | 62 | 60 << 6)
            }
        }
    }
}

fn piece_moves<const PIECE: usize, const QUIETS: bool>(move_list: &mut MoveList, occupied: u64, friendly: u64, opps: u64, mut attackers: u64) {
    let mut from: u16;
    let mut idx: usize;
    let mut attacks: u64;
    attackers &= friendly;
    while attackers > 0 {
        pop_lsb!(from, attackers);
        idx = from as usize;
        attacks = match PIECE {
            KNIGHT => KNIGHT_ATTACKS[idx],
            ROOK => rook_attacks(idx, occupied),
            BISHOP => bishop_attacks(idx, occupied),
            QUEEN => rook_attacks(idx, occupied) | bishop_attacks(idx, occupied),
            KING => KING_ATTACKS[idx],
            _ => panic!("Not a valid usize in fn piece_moves_general: {PIECE}"),
        };
        encode_moves(move_list, attacks & opps, from, MoveFlags::CAPTURE);
        if QUIETS {encode_moves(move_list, attacks & !occupied, from, MoveFlags::QUIET)}
    }
}

#[inline(always)]
fn pawn_captures(move_list: &mut MoveList, mut attackers: u64, opponents: u64, side: usize) {
    let mut from: u16;
    let mut attacks: u64;
    let mut cidx: u16;
    let mut f: u16;
    let mut promo_attackers: u64 = attackers & PENRANK[side];
    attackers &= !PENRANK[side];
    while attackers > 0 {
        pop_lsb!(from, attackers);
        attacks = PAWN_ATTACKS[side][from as usize] & opponents;
        encode_moves(move_list, attacks, from, MoveFlags::CAPTURE);
    }
    while promo_attackers > 0 {
        pop_lsb!(from, promo_attackers);
        attacks = PAWN_ATTACKS[side][from as usize] & opponents;
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
fn en_passants(move_list: &mut MoveList, pawns: u64, sq: u16, side: usize) {
    let mut attackers: u64 = PAWN_ATTACKS[side ^ 1][sq as usize] & pawns;
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
    if SIDE == WHITE {bb >> AMOUNT} else {bb << AMOUNT}
}

#[inline(always)]
fn idx_shift<const SIDE: usize, const AMOUNT: u16>(idx: u16) -> u16 {
    if SIDE == WHITE {idx + AMOUNT} else {idx - AMOUNT}
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
