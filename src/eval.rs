use std::ops::{AddAssign, Mul};
use super::{consts::*, position::{Position, bishop_attacks}};

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
const MATERIAL: [S; 5] = [S(86, 143), S(316, 266), S(338, 277), S(419, 516), S(927, 924)];
const PAWN_HT: [S; 24] = [
    S( 74, 126), S( 99, 115), S( 89,  96), S( 97,  80),
    S(-32,  47), S(  2,  41), S( 21,  18), S( 39,   9),
    S(-40, -14), S(-18, -20), S(-21, -33), S( -7, -44),
    S(-41, -34), S(-24, -32), S(-21, -47), S(-11, -51),
    S(-36, -41), S(-13, -37), S(-25, -45), S(-18, -43),
    S(-37, -38), S(-11, -31), S(-18, -36), S(-22, -37),
];
const KING_QT: [S; 16] = [
    S(-65,   8), S(-29,  26), S(-21,  35), S(-31,  38),
    S(-44,   5), S(-23,  19), S(-36,  27), S(-42,  33),
    S( 29, -23), S( 11,   0), S(-34,  19), S(-53,  23),
    S( 21, -55), S( 42, -30), S(-16,  -9), S( -1, -30),
];
const MOBILITY_KNIGHT: [S; 9] = [
    S(-34, -90), S( -8, -63), S( -1, -36),
    S(  4, -17), S( 15,  -6), S( 19,   9),
    S( 24,  13), S( 27,  19), S( 41,   8),
];
const MOBILITY_BISHOP: [S; 14] = [
    S(-15, -83), S( -6, -59), S(  1, -30), S(  6, -14), S( 10,  -1), S( 13,   8), S( 16,  17),
    S( 16,  16), S( 20,  23), S( 23,  21), S( 34,  21), S( 36,  21), S( 37,  29), S( 47,  20),
];

impl Position {
    pub fn eval(&self) -> i16 {
        let mut score: S = S(0, 0);

        // material scores
        (PAWN..=QUEEN).for_each(|i: usize| score += MATERIAL[i] * self.material[i]);

        // king
        let wk_idx: usize = (self.pieces[KING] & self.sides[WHITE]).trailing_zeros() as usize;
        let bk_idx: usize = (self.pieces[KING] & self.sides[BLACK]).trailing_zeros() as usize;
        score += KING_QT[QT_IDX[wk_idx] as usize];
        score += KING_QT[QT_IDX[bk_idx] as usize] * -1;

        // pawns
        let mut p: u64;
        let wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let bp: u64 = self.pieces[PAWN] & self.sides[BLACK];
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
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    fn mobility(&self, c: usize, opp_att: u64) -> S {
        let mut score: S = S(0, 0);
        let mut from: usize;
        let mut attacks: u64;
        let mut pieces: u64;
        let boys: u64 = self.sides[c];
        let opps: u64 = self.sides[c ^ 1];
        let safe: u64 = !boys & !opp_att;

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
        let occ: u64 = (boys | opps) ^ (self.pieces[KING] & opps) ^ self.pieces[QUEEN] ^ (self.pieces[ROOK] & opps);
        pieces = self.pieces[BISHOP] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ);
            score += MOBILITY_BISHOP[count!(attacks & safe) as usize];
        }

        score * SIDE_FACTOR[c]
    }
}