use std::{cmp::{min, max}, mem::MaybeUninit};
use super::{consts::*, position::{Position, bishop_attacks, rook_attacks}};

#[macro_export]
macro_rules! lsb {($x:expr) => {$x.trailing_zeros() as u16}}
macro_rules! pop_lsb {($idx:expr, $x:expr) => {$idx = lsb!($x); $x &= $x - 1}}

pub struct MoveList {
    pub list: [u16; 252],
    pub len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self { list: unsafe {#[allow(clippy::uninit_assumed_init, invalid_value)] MaybeUninit::uninit().assume_init()}, len: 0 }
    }
}

impl MoveList {
    #[inline(always)]
    pub fn push(&mut self, m: u16) {
        self.list[self.len] = m;
        self.len += 1;
    }
}

#[inline(always)]
fn encode_moves(move_list: &mut MoveList, mut attacks: u64, from: u16, flag: u16) {
    let f: u16 = from << 6;
    let mut aidx: u16;
    while attacks > 0 {
        pop_lsb!(aidx, attacks);
        move_list.push(flag | f | aidx);
    }
}

#[inline(always)]
fn btwn(bit1: u64, bit2: u64) -> u64 {
    (max(bit1, bit2) - min(bit1, bit2)) ^ min(bit1, bit2)
}

impl Position {
    pub fn gen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves: MoveList = MoveList::default();
        let move_list: &mut MoveList = &mut moves;
        let side: usize = usize::from(self.c);
        let occ: u64 = self.sides[0] | self.sides[1];
        let friendly: u64 = self.sides[side];
        let opps: u64 = self.sides[side ^ 1];
        let pawns: u64 = self.pieces[PAWN] & self.sides[side];
        if QUIETS {
            if self.c {pawn_pushes::<BLACK>(move_list, occ, pawns)} else {pawn_pushes::<WHITE>(move_list, occ, pawns)}
            if self.state.castle_rights & CastleRights::SIDES[side] > 0 && !self.is_in_check() {self.castles(move_list, occ)}
        }
        pawn_captures(move_list, pawns, opps, side);
        if self.state.en_passant_sq > 0 {en_passants(move_list, pawns, self.state.en_passant_sq, side)}
        piece_moves::<KNIGHT, QUIETS>(move_list, occ, friendly, opps, self.pieces[KNIGHT]);
        piece_moves::<BISHOP, QUIETS>(move_list, occ, friendly, opps, self.pieces[BISHOP]);
        piece_moves::<ROOK  , QUIETS>(move_list, occ, friendly, opps, self.pieces[ROOK]);
        piece_moves::<QUEEN , QUIETS>(move_list, occ, friendly, opps, self.pieces[QUEEN]);
        piece_moves::<KING  , QUIETS>(move_list, occ, friendly, opps, self.pieces[KING]);
        moves
    }

    fn path(&self, mut path: u64, side: usize, occ: u64) -> bool {
        let mut idx;
        while path > 0 {
            pop_lsb!(idx, path);
            if self.is_square_attacked(idx as usize, side, occ) {
                return false;
            }
        }
        true
    }

    #[inline]
    fn can_castle<const SIDE: usize>(&self, occ: u64, bit: u64, kbb: u64, kto: u64, rto: u64) -> bool {
        (occ ^ bit) & (btwn(kbb, kto) ^ kto) == 0 && (occ ^ kbb) & (btwn(bit, rto) ^ rto) == 0 && self.path(btwn(kbb, kto), SIDE, occ)
    }

    fn castles(&self, move_list: &mut MoveList, occ: u64) {
        let r = self.state.castle_rights;
        let kbb = self.pieces[KING] & self.sides[usize::from(self.c)];
        let ksq = lsb!(kbb);
        if self.c {
            if r & CastleRights::BLACK_QS > 0 && self.can_castle::<BLACK>(occ, 1 << (56 + self.castle[0]), kbb, 1 << 58, 1 << 59) {
                move_list.push(MoveFlags::QS_CASTLE | 58 | ksq << 6);
            }
            if r & CastleRights::BLACK_KS > 0 && self.can_castle::<BLACK>(occ, 1 << (56 + self.castle[1]), kbb, 1 << 62, 1 << 61) {
                move_list.push(MoveFlags::KS_CASTLE | 62 | ksq << 6);
            }
        } else {
            if r & CastleRights::WHITE_QS > 0 && self.can_castle::<WHITE>(occ, 1 << self.castle[0], kbb, 1 << 2, 1 << 3) {
                move_list.push(MoveFlags::QS_CASTLE | 2 | ksq << 6);
            }
            if r & CastleRights::WHITE_KS > 0 && self.can_castle::<WHITE>(occ, 1 << self.castle[1], kbb, 1 << 6, 1 << 5) {
                move_list.push(MoveFlags::KS_CASTLE | 6 | ksq << 6);
            }
        }
    }
}

fn piece_moves<const PIECE: usize, const QUIETS: bool>(move_list: &mut MoveList, occ: u64, friendly: u64, opps: u64, mut attackers: u64) {
    let mut from: u16;
    let mut idx: usize;
    let mut attacks: u64;
    attackers &= friendly;
    while attackers > 0 {
        pop_lsb!(from, attackers);
        idx = from as usize;
        attacks = match PIECE {
            KNIGHT => KNIGHT_ATTACKS[idx],
            ROOK => rook_attacks(idx, occ),
            BISHOP => bishop_attacks(idx, occ),
            QUEEN => rook_attacks(idx, occ) | bishop_attacks(idx, occ),
            KING => KING_ATTACKS[idx],
            _ => 0,
        };
        encode_moves(move_list, attacks & opps, from, MoveFlags::CAPTURE);
        if QUIETS {encode_moves(move_list, attacks & !occ, from, MoveFlags::QUIET)}
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

fn shift<const SIDE: usize, const AMOUNT: u8>(bb: u64) -> u64 {
    if SIDE == WHITE {bb >> AMOUNT} else {bb << AMOUNT}
}

fn idx_shift<const SIDE: usize, const AMOUNT: u16>(idx: u16) -> u16 {
    if SIDE == WHITE {idx + AMOUNT} else {idx - AMOUNT}
}

fn pawn_pushes<const SIDE: usize>(move_list: &mut MoveList, occ: u64, pawns: u64) {
    let empty: u64 = !occ;
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
