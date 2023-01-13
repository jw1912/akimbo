use std::ops::{AddAssign, Mul};
use super::{consts::*, position::{Position, rook_attacks, bishop_attacks}};

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
const MATERIAL: [S; 5] = [S(77, 120), S(315, 271), S(332, 283), S(428, 510), S(938, 917)];
const KING_QT: [S; 16] = [
    S(-56,   8), S(-17,  26), S(-10,  37), S(-26,  41),
    S(-50,   7), S(-17,  19), S(-22,  29), S(-23,  33),
    S(  7, -14), S( -6,   6), S(-32,  23), S(-43,  25),
    S( 16, -55), S( 32, -26), S( -9,  -6), S(  7, -18),
];
const PAWN_HT: [S; 24] = [
    S( 20,  85), S( 40,  76), S( 41,  52), S( 56,  33),
    S(-13,   9), S(  8,   1), S( 27, -21), S( 23, -46),
    S(-27,  -7), S( -6, -12), S(-13, -26), S(  1, -40),
    S(-28, -19), S(-12, -14), S( -9, -30), S( -1, -36),
    S(-20, -25), S(  2, -22), S(-14, -27), S( -8, -28),
    S(-27, -24), S( -4, -17), S(-17, -18), S(-23, -18),
];
const MOBILITY_KNIGHT: [S; 9] = [
    S(-36, -82), S( -8, -54), S(  1, -31),
    S(  6, -15), S( 13,  -4), S( 17,  12),
    S( 21,  17), S( 23,  23), S( 36,  13),
];
const MOBILITY_BISHOP: [S; 14] = [
    S(-14, -73), S( -3, -52), S(  5, -30), S( 10, -15), S( 14,  -2), S( 15,   7), S( 18,  15),
    S( 18,  17), S( 21,  24), S( 22,  24), S( 34,  24), S( 36,  25), S( 36,  34), S( 52,  28),
];
const MOBILITY_ROOK: [S; 15] = [
    S(-30, -89), S(-13, -51), S(-13, -33), S(-13, -19), S(-10, -12),
    S(-11,  -2), S( -7,   2), S( -2,   4), S(  4,  10), S( 11,  11),
    S( 14,  16), S( 17,  19), S( 23,  20), S( 27,  18), S( 21,  21),
];
const MOBILITY_QUEEN: [S; 28] = [
    S( -1, -43), S( -4, -54), S( -2, -62), S( -2, -64), S(  1, -76), S(  2, -55), S(  0, -41),
    S( -2, -21), S( -1, -20), S( -1,  -9), S( -3,   5), S( -4,  15), S( -5,  23), S( -2,  24),
    S( -8,  37), S(-10,  42), S( -4,  45), S(  0,  38), S(  4,  45), S( 17,  41), S( 25,  38),
    S( 37,  39), S( 24,  43), S( 52,  38), S( 28,  40), S( 56,  49), S( 31,  29), S( 28,  32),
];
const PAWN_PASSED: [S; 6] = [S(-2, -9), S(-7, -2), S(-8, 20), S(10, 43), S(19, 93), S(36, 76)];
const PAWN_SHIELD: S = S(18, -4);
const KING_LINEAR: S = S(-3, -5);
const KING_QUADRATIC: S = S(-6, 3);

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
        score += self.mobility(WHITE, bp_att, bk_sqs);
        score += self.mobility(BLACK, wp_att, wk_sqs);

        // taper eval
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    fn mobility(&self, c: usize, opp_att: u64, k_sqs: u64) -> S {
        let mut score: S = S(0, 0);
        let mut danger: i16 = 0;
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
            danger += count!(attacks & k_sqs);
        }

        // bishop mobility
        // - ignore friendly queens
        // - ignore enemy queens and rooks
        let mut occ: u64 = (boys | opps) ^ (self.pieces[KING] & opps) ^ self.pieces[QUEEN] ^ (self.pieces[ROOK] & opps);
        pieces = self.pieces[BISHOP] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ);
            score += MOBILITY_BISHOP[count!(attacks & safe) as usize];
            danger += count!(attacks & k_sqs);
        }

        // rook mobility
        // - ignore friendly queens and rooks
        // - ignore enemy queens
        occ ^= self.pieces[ROOK];
        pieces = self.pieces[ROOK] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = rook_attacks(from, occ);
            score += MOBILITY_ROOK[count!(attacks & safe) as usize];
            danger += count!(attacks & k_sqs);
        }

        // queen
        // - ignore friendly queens, rooks and bishops
        occ ^= (self.pieces[QUEEN] & opps) ^ (self.pieces[BISHOP] & boys);
        pieces = self.pieces[QUEEN] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ) | rook_attacks(from, occ);
            score += MOBILITY_QUEEN[count!(attacks & safe) as usize];
            danger += count!(attacks & k_sqs);
        }

        // threat to opposite king
        score += KING_LINEAR    * -danger;
        score += KING_QUADRATIC * -danger.pow(2);

        score * SIDE_FACTOR[c]
    }
}