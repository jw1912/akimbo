use std::ops::{AddAssign, Mul};
use super::{consts::*, position::{Position, rook_attacks, bishop_attacks}};

macro_rules! count {($bb:expr) => {($bb).count_ones() as i16}}
macro_rules! lsb {($x:expr) => {($x).trailing_zeros() as usize}}
macro_rules! pull_lsb {($idx:expr, $x:expr) => {$idx = lsb!($x); $x &= $x - 1}}

#[derive(Clone, Copy, Debug, Default)]
pub struct S(pub i16, pub i16);

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
pub const LAZY_MATERIAL: [S; 5] = [S(75, 113), S(318, 294), S(331, 308), S(450, 508), S(944, 945)];

// eval values
pub const MATERIAL: [S; 5] = [S(85, 125), S(312, 268), S(332, 279), S(432, 498), S(927, 945)];
pub const PAWN_PST: [S; 24] = [
    S( 79, 114), S( 92, 109), S( 85,  87), S( 84,  77),
    S(-26,  43), S(  6,  38), S( 26,  20), S( 39,   8),
    S(-34, -15), S(-11, -20), S(-18, -27), S( -4, -40),
    S(-34, -35), S(-16, -32), S(-13, -44), S( -7, -48),
    S(-25, -43), S( -1, -40), S(-17, -45), S(-13, -41),
    S(-32, -41), S( -8, -34), S(-19, -35), S(-28, -31),
];
pub const MOBILITY_KNIGHT: [S; 9] = [S(-34, -74), S(-7, -54), S(3, -34), S(9, -18), S(15, -5), S(18, 13), S(22, 17), S(21, 27), S(34, 15)];
pub const MOBILITY_BISHOP: [S; 14] = [S(-16, -74), S(-2, -57), S(5, -33), S(10, -17), S(14, -4), S(15, 6), S(17, 14), S(16, 17), S(19, 24), S(18, 25), S(34, 26), S(35, 27), S(43, 31), S(40, 31)];
pub const MOBILITY_ROOK: [S; 15] = [S(-31, -84), S(-15, -56), S(-18, -33), S(-17, -17), S(-11, -10), S(-11, -3), S(-9, 3), S(-5, 6), S(0, 13), S(7, 15), S(11, 18), S(11, 22), S(13, 25), S(19, 24), S(20, 25)];
pub const PAWN_SHIELD: S = S(22, -3);
pub const PAWN_PASSED: S = S(-5, 26);
pub const KING_LINEAR: S = S(-9, 0);
pub const KING_QUADRATIC: S = S(-6, 3);

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

        // king position
        let wk_idx: usize = (self.pieces[KING] & self.sides[WHITE]).trailing_zeros() as usize;
        let bk_idx: usize = (self.pieces[KING] & self.sides[BLACK]).trailing_zeros() as usize;
        let wk_sqs: u64 = KING_ATTACKS[wk_idx];
        let bk_sqs: u64 = KING_ATTACKS[bk_idx];

        // pawn bitboards
        let wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let bp: u64 = self.pieces[PAWN] & self.sides[BLACK];
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);

        // pawns
        score += PAWN_SHIELD * (count!(wp & wk_sqs) - count!(bp & bk_sqs));
        score += PAWN_PASSED * (count!(wp & !bspans(bp | bp_att)) - count!(bp & !wspans(wp | wp_att)));
        let mut p: u64 = wp;
        while p > 0 {
            score += PAWN_PST[PST_IDX[56 ^ lsb!(p)] as usize];
            p &= p - 1;
        }
        p = bp;
        while p > 0 {
            score += PAWN_PST[PST_IDX[lsb!(p)] as usize] * -1;
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
        // only count threats to king as it is a volatile piece
        // - ignore friendly queens, rooks and bishops
        occ ^= (self.pieces[QUEEN] & opps) ^ (self.pieces[BISHOP] & boys);
        pieces = self.pieces[QUEEN] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ) | rook_attacks(from, occ);
            danger += count!(attacks & k_sqs);
        }

        // threat to opposite king
        score += KING_LINEAR    * -danger;
        score += KING_QUADRATIC * -danger.pow(2);

        score * SIDE_FACTOR[c]
    }
}