use std::mem::MaybeUninit;
use super::{consts::*, position::{Pos, bishop_attacks, rook_attacks}};

#[macro_export]
macro_rules! lsb {($x:expr) => {$x.trailing_zeros() as u16}}
macro_rules! pop_lsb {($idx:expr, $x:expr) => {$idx = lsb!($x); $x &= $x - 1}}

pub struct MoveList {
    pub list: [u16; 252],
    pub len: usize,
}

impl MoveList {
    pub fn uninit() -> Self {
        Self { list: unsafe {#[allow(clippy::uninit_assumed_init, invalid_value)] MaybeUninit::uninit().assume_init()}, len: 0 }
    }

    #[inline(always)]
    pub fn push(&mut self, m: u16) {
        self.list[self.len] = m;
        self.len += 1;
    }
}

#[inline(always)]
fn encode_moves(move_list: &mut MoveList, mut attacks: u64, from: u16, flag: u16) {
    let fr = from << 6;
    let mut to;
    while attacks > 0 {
        pop_lsb!(to, attacks);
        move_list.push(flag | fr | to);
    }
}

impl Pos {
    pub fn gen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::uninit();
        let move_list = &mut moves;
        let side = usize::from(self.c);
        let occ = self.sides[0] | self.sides[1];
        let friendly = self.sides[side];
        let opps = self.sides[side ^ 1];
        let pawns = self.pieces[PAWN] & self.sides[side];
        if QUIETS {
            if self.c {pawn_pushes::<BLACK>(move_list, occ, pawns)} else {pawn_pushes::<WHITE>(move_list, occ, pawns)}
            if self.state.cr & CS[side] > 0 && !self.in_check() {self.castles(move_list, occ)}
        }
        pawn_captures(move_list, pawns, opps, side);
        if self.state.enp > 0 {en_passants(move_list, pawns, self.state.enp, side)}
        piece_moves::<KNIGHT, QUIETS>(move_list, occ, friendly, opps, self.pieces[KNIGHT]);
        piece_moves::<BISHOP, QUIETS>(move_list, occ, friendly, opps, self.pieces[BISHOP]);
        piece_moves::<ROOK  , QUIETS>(move_list, occ, friendly, opps, self.pieces[ROOK]);
        piece_moves::<QUEEN , QUIETS>(move_list, occ, friendly, opps, self.pieces[QUEEN]);
        piece_moves::<KING  , QUIETS>(move_list, occ, friendly, opps, self.pieces[KING]);
        moves
    }

    fn castles(&self, moves: &mut MoveList, occ: u64) {
        if self.c {
            if self.state.cr & BQS > 0 && occ & B8C8D8 == 0 && !self.is_sq_att(59, BLACK, occ) {moves.push(60 << 6 | 58 | QS)}
            if self.state.cr & BKS > 0 && occ & F8G8 == 0 && !self.is_sq_att(61, BLACK, occ) {moves.push(60 << 6 | 62 | KS)}
        } else {
            if self.state.cr & WQS > 0 && occ & B1C1D1 == 0 && !self.is_sq_att(3, WHITE, occ) {moves.push(4 << 6 | 2 | QS)}
            if self.state.cr & WKS > 0 && occ & F1G1 == 0 && !self.is_sq_att(5, WHITE, occ) {moves.push(4 << 6 | 6 | KS)}
        }
    }
}

fn piece_moves<const PIECE: usize, const QUIETS: bool>(move_list: &mut MoveList, occ: u64, friendly: u64, opps: u64, mut attackers: u64) {
    let mut from;
    let mut idx;
    let mut attacks;
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
        encode_moves(move_list, attacks & opps, from, CAP);
        if QUIETS {encode_moves(move_list, attacks & !occ, from, QUIET)}
    }
}

#[inline(always)]
fn pawn_captures(move_list: &mut MoveList, mut attackers: u64, opponents: u64, side: usize) {
    let mut from;
    let mut attacks;
    let mut cidx;
    let mut f;
    let mut promo_attackers = attackers & PENRANK[side];
    attackers &= !PENRANK[side];
    while attackers > 0 {
        pop_lsb!(from, attackers);
        attacks = PAWN_ATTACKS[side][from as usize] & opponents;
        encode_moves(move_list, attacks, from, CAP);
    }
    while promo_attackers > 0 {
        pop_lsb!(from, promo_attackers);
        attacks = PAWN_ATTACKS[side][from as usize] & opponents;
        while attacks > 0 {
            pop_lsb!(cidx, attacks);
            f = from << 6;
            move_list.push(QPC  | cidx | f);
            move_list.push(NPC | cidx | f);
            move_list.push(BPC | cidx | f);
            move_list.push(RPC | cidx | f);
        }
    }
}

#[inline(always)]
fn en_passants(move_list: &mut MoveList, pawns: u64, sq: u16, side: usize) {
    let mut attackers = PAWN_ATTACKS[side ^ 1][sq as usize] & pawns;
    let mut cidx;
    while attackers > 0 {
        pop_lsb!(cidx, attackers);
        move_list.push( ENP | sq | cidx << 6 );
    }
}

fn shift<const SIDE: usize, const AMOUNT: u8>(bb: u64) -> u64 {
    if SIDE == WHITE {bb >> AMOUNT} else {bb << AMOUNT}
}

fn idx_shift<const SIDE: usize, const AMOUNT: u16>(idx: u16) -> u16 {
    if SIDE == WHITE {idx + AMOUNT} else {idx - AMOUNT}
}

fn pawn_pushes<const SIDE: usize>(move_list: &mut MoveList, occ: u64, pawns: u64) {
    let empty = !occ;
    let mut pushable_pawns = shift::<SIDE, 8>(empty) & pawns;
    let mut dbl_pushable_pawns = shift::<SIDE, 8>(shift::<SIDE, 8>(empty & DBLRANK[SIDE]) & empty) & pawns;
    let mut promotable_pawns = pushable_pawns & PENRANK[SIDE];
    pushable_pawns &= !PENRANK[SIDE];
    let mut idx;
    while pushable_pawns > 0 {
        pop_lsb!(idx, pushable_pawns);
        move_list.push(idx_shift::<SIDE, 8>(idx) | idx << 6);
    }
    while promotable_pawns > 0 {
        pop_lsb!(idx, promotable_pawns);
        let to = idx_shift::<SIDE, 8>(idx);
        let f = idx << 6;
        move_list.push(QPR | to | f);
        move_list.push( PR | to | f);
        move_list.push(BPR | to | f);
        move_list.push(RPR | to | f);
    }
    while dbl_pushable_pawns > 0 {
        pop_lsb!(idx, dbl_pushable_pawns);
        move_list.push(DBL | idx_shift::<SIDE, 16>(idx) | idx << 6);
    }
}
