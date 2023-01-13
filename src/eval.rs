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

// lazy eval values
const LAZY_MATERIAL: [S; 5] = [S(75, 113), S(318, 294), S(331, 308), S(450, 508), S(944, 945)];

// eval values
const MATERIAL: [S; 5] = [S(77, 120), S(316, 268), S(337, 277), S(418, 526), S(931, 939)];
const KING_QT: [S; 16] = [
    S(-61,   7), S(-32,  28), S(-32,  39), S(-40,  42),
    S(-49,   8), S(-33,  22), S(-45,  32), S(-44,  35),
    S( 23, -19), S( -3,   5), S(-36,  22), S(-48,  25),
    S( 30, -57), S( 39, -27), S( -5, -11), S( 15, -30),
];
const PAWN_HT: [S; 24] = [
    S( 24,  85), S( 40,  79), S( 46,  53), S( 61,  33),
    S(-23,   9), S(  8,   1), S( 22, -22), S( 25, -46),
    S(-30,  -8), S( -8, -13), S(-11, -27), S(  2, -40),
    S(-31, -21), S(-13, -16), S( -9, -30), S( -1, -36),
    S(-25, -27), S(  0, -23), S(-16, -28), S(-10, -27),
    S(-32, -26), S( -5, -18), S(-17, -20), S(-24, -18),
];
const MOBILITY_KNIGHT: [S; 9] = [
    S(-38, -82), S( -9, -60), S( -1, -36),
    S(  5, -15), S( 16,  -4), S( 20,  13),
    S( 26,  16), S( 29,  22), S( 40,  12),
];
const MOBILITY_BISHOP: [S; 14] = [
    S(-14, -79), S( -4, -56), S(  4, -29), S(  9, -10), S( 12,   3), S( 16,  11), S( 19,  21),
    S( 20,  22), S( 22,  28), S( 26,  28), S( 36,  28), S( 39,  28), S( 39,  38), S( 43,  32),
];
const PAWN_PASSED: [S; 6] = [S(1, -7), S(-8, 0), S(-10, 22), S(10, 43), S(27, 91), S(45, 74)];
const PAWN_SHIELD: S = S(20, -4);

#[inline(always)]
fn wspans(mut pwns: u64) -> u64 {
    pwns |= pwns << 8;
    pwns |= pwns << 16;
    pwns |= pwns << 32;
    pwns
}

#[inline(always)]
fn bspans(mut pwns: u64) -> u64 {
    pwns |= pwns >> 8;
    pwns |= pwns >> 16;
    pwns |= pwns >> 32;
    pwns
}

impl Position {
    #[inline]
    pub fn lazy_eval(&self) -> i16 {
        // material-only eval
        let mut score: S = S(0, 0);
        (PAWN..=QUEEN).for_each(|i: usize| score += LAZY_MATERIAL[i] * self.material[i]);

        // taper eval
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    pub fn eval(&self) -> i16 {
        let mut score: S = S(0, 0);

        // material scores
        (PAWN..=QUEEN).for_each(|i: usize| score += MATERIAL[i] * self.material[i]);

        // king
        let wk_idx: usize = (self.pieces[KING] & self.sides[WHITE]).trailing_zeros() as usize;
        let bk_idx: usize = (self.pieces[KING] & self.sides[BLACK]).trailing_zeros() as usize;
        let wk_sqs: u64 = KING_ATTACKS[wk_idx];
        let bk_sqs: u64 = KING_ATTACKS[bk_idx];
        score += KING_QT[KING_IDX[wk_idx] as usize];
        score += KING_QT[KING_IDX[bk_idx] as usize] * -1;

        // pawns
        let mut p: u64;
        let wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let bp: u64 = self.pieces[PAWN] & self.sides[BLACK];
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);
        score += PAWN_SHIELD * (count!(wp & wk_sqs) - count!(bp & bk_sqs));
        p = wp & !bspans(bp | bp_att); // white passed pawns
        while p > 0 {
            score += PAWN_PASSED[lsb!(p) / 8 - 1];
            p &= p - 1;
        }
        p = bp & !wspans(wp | wp_att); // black passed pawns
        while p > 0 {
            score += PAWN_PASSED[6 - lsb!(p) / 8] * -1;
            p &= p - 1;
        }
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