use super::{util::*, position::*};
use std::sync::atomic::Ordering::Relaxed;

macro_rules! bitloop {($bb:expr, $sq:ident, $func:expr) => {
    while $bb > 0 {
        let $sq = $bb.trailing_zeros() as u8;
        $bb &= $bb - 1;
        $func;
    }
}}

pub struct List<T> {
    pub list: [T; 252],
    pub len: usize,
}

pub type MoveList = List<Move>;
pub type ScoreList = List<i16>;

impl<T: Copy + Default> Default for List<T> {
    fn default() -> Self {
        Self { list: [T::default(); 252], len: 0 }
    }
}

impl MoveList {
    #[inline]
    pub fn push(&mut self, from: u8, to: u8, flag: u8, mpc: usize) {
        self.list[self.len] = Move { from, to, flag, pc: mpc as u8 };
        self.len += 1;
    }

    pub fn pick(&mut self, scores: &mut ScoreList) -> Option<(Move, i16)> {
        if self.len == 0 { return None }
        let (mut idx, mut best) = (0, i16::MIN);
        for i in 0..self.len {
            let score = scores.list[i];
            if score > best {
                best = score;
                idx = i;
            }
        }
        self.len -= 1;
        scores.list.swap(idx, self.len);
        self.list.swap(idx, self.len);
        Some((self.list[self.len], best))
    }
}

#[inline]
fn encode<const FLAG: u8>(moves: &mut MoveList, mut attacks: u64, from: u8, pc: usize) {
    bitloop!(attacks, to, moves.push(from, to, FLAG, pc))
}

impl Position {
    pub fn gen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::default();
        let (side, occ) = (usize::from(self.c), self.bb[0] | self.bb[1]);
        let (boys, opps) = (self.bb[side], self.bb[side ^ 1]);
        let pawns = self.bb[Piece::PAWN] & boys;

        // special quiet moves
        if QUIETS {
            let r = self.rights;
            if r & [Rights::WHITE, Rights::BLACK][side] > 0 && !self.in_check() {
                let kbb = self.bb[Piece::KING] & self.bb[side];
                let ksq = kbb.trailing_zeros() as u8;
                if self.c {
                    if self.castle(Rights::BQS, 0, occ, kbb, 1 << 58, 1 << 59) {moves.push(ksq, 58, Flag::QS, Piece::KING)}
                    if self.castle(Rights::BKS, 1, occ, kbb, 1 << 62, 1 << 61) {moves.push(ksq, 62, Flag::KS, Piece::KING)}
                } else {
                    if self.castle(Rights::WQS, 0, occ, kbb, 1 << 2, 1 << 3) {moves.push(ksq, 2, Flag::QS, Piece::KING)}
                    if self.castle(Rights::WKS, 1, occ, kbb, 1 << 6, 1 << 5) {moves.push(ksq, 6, Flag::KS, Piece::KING)}
                }
            }

            // pawn pushes
            let empty = !occ;
            let mut dbl = shift(side, shift(side, empty & DBLRANK[side]) & empty) & pawns;
            let mut push = shift(side, empty) & pawns;
            let mut promo = push & PENRANK[side];
            push &= !PENRANK[side];
            bitloop!(push, from, moves.push(from, idx_shift::<8>(side, from), Flag::QUIET, Piece::PAWN));
            bitloop!(promo, from,
                for flag in Flag::PROMO..=Flag::QPR {moves.push(from, idx_shift::<8>(side, from), flag, Piece::PAWN)}
            );
            bitloop!(dbl, from, moves.push(from, idx_shift::<16>(side, from), Flag::DBL, Piece::PAWN));
        }

        // pawn captures
        if self.enp_sq > 0 {
            let mut attackers = Attacks::PAWN[side ^ 1][self.enp_sq as usize] & pawns;
            bitloop!(attackers, from, moves.push(from, self.enp_sq, Flag::ENP, Piece::PAWN));
        }
        let (mut attackers, mut promo) = (pawns & !PENRANK[side], pawns & PENRANK[side]);
        bitloop!(attackers, from,
            encode::<{ Flag::CAP }>(&mut moves, Attacks::PAWN[side][from as usize] & opps, from, Piece::PAWN)
        );
        bitloop!(promo, from, {
            let mut attacks = Attacks::PAWN[side][from as usize] & opps;
            bitloop!(attacks, to, for flag in Flag::NPC..=Flag::QPC { moves.push(from, to, flag, Piece::PAWN) });
        });

        // non-pawn moves
        for pc in Piece::KNIGHT..=Piece::KING {
            let mut attackers = boys & self.bb[pc];
            bitloop!(attackers, from, {
                let attacks = match pc {
                    Piece::KNIGHT => Attacks::KNIGHT[from as usize],
                    Piece::ROOK   => Attacks::rook  (from as usize, occ),
                    Piece::BISHOP => Attacks::bishop(from as usize, occ),
                    Piece::QUEEN  => Attacks::bishop(from as usize, occ) | Attacks::rook(from as usize, occ),
                    Piece::KING   => Attacks::KING  [from as usize],
                    _ => unreachable!(),
                };
                encode::<{ Flag::CAP }>(&mut moves, attacks & opps, from, pc);
                if QUIETS { encode::<{ Flag::QUIET }>(&mut moves, attacks & !occ, from, pc) }
            });
        }
        moves
    }

    fn path(&self, side: usize, mut path: u64, occ: u64) -> bool {
        bitloop!(path, idx, if self.sq_attacked(idx as usize, side, occ) {return false});
        true
    }

    fn castle(&self, right: u8, ks: usize, occ: u64, kbb: u64, kto: u64, rto: u64) -> bool {
        let side = usize::from(self.c);
        let bit = 1 << (56 * side + usize::from(ROOK_FILES[side][ks].load(Relaxed)));
        self.rights & right > 0
            && (occ ^ bit) & (btwn(kbb, kto) ^ kto) == 0
            && (occ ^ kbb) & (btwn(bit, rto) ^ rto) == 0
            && self.path(side, btwn(kbb, kto), occ)
    }
}

fn shift(side: usize,bb: u64) -> u64 {
    if side == Side::WHITE {bb >> 8} else {bb << 8}
}

fn idx_shift<const AMOUNT: u8>(side: usize, idx: u8) -> u8 {
    if side == Side::WHITE {idx + AMOUNT} else {idx - AMOUNT}
}

fn btwn(bit1: u64, bit2: u64) -> u64 {
    let min = bit1.min(bit2);
    (bit1.max(bit2) - min) ^ min
}

