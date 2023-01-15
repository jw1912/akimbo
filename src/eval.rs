use std::ops::{AddAssign, Mul};
use super::{consts::*, position::{Position, bishop_attacks, rook_attacks}};

macro_rules! count {($bb:expr) => {($bb).count_ones() as i16}}
macro_rules! lsb {($x:expr) => {($x).trailing_zeros() as usize}}
macro_rules! pull_lsb {($idx:expr, $x:expr) => {$idx = lsb!($x); $x &= $x - 1}}

#[derive(Clone, Copy)]
struct S(i16, i16);

impl AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Mul<i16> for S {
    type Output = S;
    fn mul(self, rhs: i16) -> Self::Output {
        S(self.0 * rhs, self.1 * rhs)
    }
}

// eval values
const MATERIAL: [S; 5] = [S(83, 145), S(316, 266), S(337, 280), S(432, 493), S(912, 942)];
const PAWN_HT: [S; 24] = [
    S( 67, 126), S( 84, 119), S( 79,  94), S( 85,  78),
    S(-28,  47), S( -3,  42), S( 17,  19), S( 36,  10),
    S(-37, -14), S(-15, -22), S(-21, -33), S( -8, -45),
    S(-36, -35), S(-19, -34), S(-15, -49), S( -9, -52),
    S(-26, -43), S( -5, -41), S(-19, -47), S(-14, -45),
    S(-32, -41), S(-11, -33), S(-22, -36), S(-29, -32),
];
const KING_QT: [S; 16] = [
    S(-72,  13), S(-37,  31), S(-31,  41), S(-45,  45),
    S(-51,  10), S(-40,  25), S(-48,  33), S(-49,  38),
    S( 16, -16), S(-10,   8), S(-41,  24), S(-53,  26),
    S( 28, -55), S( 42, -27), S( -3,  -7), S( 13, -22),
];
const MOBILITY_KNIGHT: [S; 9] = [
    S(-37, -87), S(-10, -58), S(  0, -33),
    S(  6, -17), S( 14,  -6), S( 18,  11),
    S( 23,  14), S( 25,  21), S( 39,  10),
];
const MOBILITY_BISHOP: [S; 14] = [
    S(-17, -72), S( -5, -55), S(  2, -33), S(  8, -18), S( 12,  -6), S( 15,   3), S( 18,  12),
    S( 19,  13), S( 23,  20), S( 23,  20), S( 36,  19), S( 45,  19), S( 54,  24), S( 58,  19),
];
const MOBILITY_ROOK: [S; 15] = [
    S(-35, -73), S(-21, -41), S(-18, -23), S(-13, -17), S(-13,  -7),
    S(-10,   4), S( -6,  10), S(  1,  11), S(  7,  17), S( 18,  16),
    S( 21,  19), S( 24,  23), S( 32,  22), S( 42,  19), S( 27,  25),
];
const PAWN_SHIELD: S = S(19, -5);

impl Position {
    pub fn eval(&self) -> i16 {
        let mut score: S = S(0, 0);
        let wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let bp: u64 = self.pieces[PAWN] & self.sides[BLACK];

        // material scores
        (PAWN..=QUEEN).for_each(|i: usize| score += MATERIAL[i] * self.material[i]);

        // king
        let wk_idx: usize = (self.pieces[KING] & self.sides[WHITE]).trailing_zeros() as usize;
        let bk_idx: usize = (self.pieces[KING] & self.sides[BLACK]).trailing_zeros() as usize;
        score += KING_QT[QT_IDX[wk_idx] as usize];
        score += KING_QT[QT_IDX[bk_idx] as usize] * -1;
        score += PAWN_SHIELD * (count!(wp & KING_ATTACKS[wk_idx]) - count!(bp & KING_ATTACKS[bk_idx]));

        // pawns
        let mut p: u64;
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);
        p = wp; // white pst bonuses
        while p > 0 {
            score += PAWN_HT[PAWN_IDX[56 ^ lsb!(p)] as usize];
            p &= p - 1;
        }
        p = bp; // black pst bonuses
        while p > 0 {
            score += PAWN_HT[PAWN_IDX[lsb!(p)] as usize] * -1;
            p &= p - 1;
        }

        // mobility
        score += self.mobility(WHITE, bp_att);
        score += self.mobility(BLACK, wp_att);

        // taper eval
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    fn mobility(&self, c: usize, opp_att: u64) -> S {
        let mut score: S = S(0, 0);
        let mut from: usize;
        let mut attacks: u64;
        let mut pieces: u64;
        let boys: u64 = self.sides[c];
        let opps: u64 = self.sides[c ^ 1];
        let safe: u64 = !boys & !opp_att;
        let rooks: u64 = self.pieces[ROOK];

        // knight mobility
        pieces = self.pieces[KNIGHT] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = KNIGHT_ATTACKS[from];
            score += MOBILITY_KNIGHT[count!(attacks & safe) as usize];
        }

        // bishop mobility
        // - ignore friendly queens
        // - ignore enemy queens and rooks
        let mut occ: u64 = (boys | opps) ^ (self.pieces[KING] & opps) ^ self.pieces[QUEEN] ^ (rooks & opps);
        pieces = self.pieces[BISHOP] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ);
            score += MOBILITY_BISHOP[count!(attacks & safe) as usize];
        }

        // rook mobility
        // ingore friendly rooks and queens
        // ignore enemy queens
        occ ^= rooks;
        pieces = rooks & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = rook_attacks(from, occ);
            score += MOBILITY_ROOK[count!(attacks & safe) as usize];
        }

        score * SIDE[c]
    }
}