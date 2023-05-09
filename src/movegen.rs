use super::{consts::*, position::{Position, Move, ratt, batt}};

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
        self.list[self.len] = Move { from, to, flag, mpc: mpc as u8 };
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
        let pawns = self.bb[P] & boys;

        // special quiet moves
        if QUIETS {
            if self.cr & CS[side] > 0 && !self.in_check() {
                if self.c {
                    if self.cr & BQS > 0 && occ & BD8 == 0 && !self.is_sq_att(59, BL, occ) {moves.push(60, 58, QS, K)}
                    if self.cr & BKS > 0 && occ & FG8 == 0 && !self.is_sq_att(61, BL, occ) {moves.push(60, 62, KS, K)}
                } else {
                    if self.cr & WQS > 0 && occ & BD1 == 0 && !self.is_sq_att(3, WH, occ) {moves.push(4, 2, QS, K)}
                    if self.cr & WKS > 0 && occ & FG1 == 0 && !self.is_sq_att(5, WH, occ) {moves.push(4, 6, KS, K)}
                }
            }

            // pawn pushes
            let empty = !occ;
            let mut dbl = shift(side, shift(side, empty & DBLRANK[side]) & empty) & pawns;
            let mut push = shift(side, empty) & pawns;
            let mut promo = push & PENRANK[side];
            push &= !PENRANK[side];
            bitloop!(push, from, moves.push(from, idx_shift::<8>(side, from), QUIET, P));
            bitloop!(promo, from, for flag in NPR..=QPR {moves.push(from, idx_shift::<8>(side, from), flag, P)});
            bitloop!(dbl, from, moves.push(from, idx_shift::<16>(side, from), DBL, P));
        }

        // pawn captures
        if self.enp > 0 {
            let mut attackers = PATT[side ^ 1][self.enp as usize] & pawns;
            bitloop!(attackers, from, moves.push(from, self.enp, ENP, P));
        }
        let (mut attackers, mut promo) = (pawns & !PENRANK[side], pawns & PENRANK[side]);
        bitloop!(attackers, from, encode::<CAP>(&mut moves, PATT[side][from as usize] & opps, from, P));
        bitloop!(promo, from, {
            let mut attacks = PATT[side][from as usize] & opps;
            bitloop!(attacks, to, for flag in NPC..=QPC { moves.push(from, to, flag, P) });
        });

        // non-pawn moves
        for pc in N..=K {
            let mut attackers = boys & self.bb[pc];
            bitloop!(attackers, from, {
                let attacks = match pc {
                    N => NATT[from as usize],
                    R => ratt(from as usize, occ),
                    B => batt(from as usize, occ),
                    Q => ratt(from as usize, occ) | batt(from as usize, occ),
                    K => KATT[from as usize],
                    _ => 0
                };
                encode::<CAP>(&mut moves, attacks & opps, from, pc);
                if QUIETS { encode::<QUIET>(&mut moves, attacks & !occ, from, pc) }
            });
        }
        moves
    }
}

fn shift(side: usize,bb: u64) -> u64 {
    if side == WH {bb >> 8} else {bb << 8}
}

fn idx_shift<const AMOUNT: u8>(side: usize, idx: u8) -> u8 {
    if side == WH {idx + AMOUNT} else {idx - AMOUNT}
}

