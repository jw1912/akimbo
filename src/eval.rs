use std::{cmp::min, ops::{AddAssign, Mul}};
use super::{consts::*, position::{Pos, bishop_attacks, rook_attacks}};

macro_rules! count {($bb:expr) => {($bb).count_ones() as usize}}
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
const MATERIAL: [S; 5] = [S(86, 142), S(319, 262), S(339, 275), S(433, 491), S(909, 941)];
static PAWN_HT: [S; 24] = [
    S( 64, 127), S( 75, 123), S( 79,  95), S( 87,  78),
    S(-29,  47), S( -4,  44), S( 17,  20), S( 33,  12),
    S(-40, -12), S(-18, -19), S(-24, -32), S( -9, -43),
    S(-39, -33), S(-23, -31), S(-20, -46), S(-11, -50),
    S(-30, -40), S(-11, -37), S(-22, -45), S(-16, -43),
    S(-30, -39), S(-10, -31), S(-17, -35), S(-22, -35),
];
static KING_QT: [S; 16] = [
    S(-63,   8), S(-38,  29), S(-27,  38), S(-40,  41),
    S(-47,   7), S(-26,  20), S(-38,  28), S(-50,  36),
    S( 21, -19), S(  3,   3), S(-40,  22), S(-59,  25),
    S( 18, -53), S( 45, -31), S(-12,  -7), S( -2, -20),
];
static MOBILITY_KNIGHT: [S; 9] = [
    S(-35, -80), S( -9, -54), S(  0, -33),
    S(  5, -15), S( 13,  -4), S( 17,  13),
    S( 21,  17), S( 24,  23), S( 38,  12),
];
static MOBILITY_BISHOP: [S; 14] = [
    S(-16, -66), S( -6, -50), S(  1, -29), S(  6, -14), S( 10,  -1), S( 13,   8), S( 16,  16),
    S( 17,  17), S( 19,  24), S( 21,  23), S( 36,  23), S( 48,  21), S( 53,  26), S( 54,  24),
];
static MOBILITY_ROOK: [S; 15] = [
    S(-36, -67), S(-20, -43), S(-17, -25), S(-12, -18), S(-11,  -9),
    S( -9,   2), S( -5,   9), S(  2,  10), S(  8,  15), S( 20,  15),
    S( 21,  18), S( 26,  21), S( 30,  22), S( 41,  19), S( 30,  23),
];

impl Pos {
    pub fn eval(&self) -> i16 {
        // draws: KvK, KvKB, KvKN
        if self.material_draw() {return 0}

        let mut score = S(0, 0);

        // pawn bitboards
        let wp = self.pieces[PAWN] & self.sides[WHITE];
        let bp = self.pieces[PAWN] & self.sides[BLACK];
        let wp_att = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);

        // material scores
        (PAWN..=QUEEN).for_each(|i: usize| score += MATERIAL[i] * self.material[i]);

        // king quarter table
        score += KING_QT[QT_IDX[lsb!(self.pieces[KING] & self.sides[WHITE])] as usize];
        score += KING_QT[QT_IDX[lsb!(self.pieces[KING] & self.sides[BLACK])] as usize] * -1;

        // pawn half table
        let mut p = wp; // white pst bonuses
        while p > 0 {
            score += PAWN_HT[PAWN_IDX[56 ^ lsb!(p)] as usize];
            p &= p - 1;
        }
        p = bp; // black pst bonuses
        while p > 0 {
            score += PAWN_HT[PAWN_IDX[lsb!(p)] as usize] * -1;
            p &= p - 1;
        }

        // knight, bishop and rook mobility
        score += self.mobility(WHITE, bp_att);
        score += self.mobility(BLACK, wp_att) * -1;

        // taper eval
        let phase = min(self.phase as i32, TPHASE);
        let score = ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16;
        SIDE[usize::from(self.c)] * score
    }

    fn mobility(&self, c: usize, opp_att: u64) -> S {
        let mut score= S(0, 0);
        let mut from;
        let mut attacks;
        let mut pieces;
        let boys = self.sides[c];
        let opps = self.sides[c ^ 1];
        let safe = !boys & !opp_att;
        let rooks = self.pieces[ROOK];

        // knight mobility
        pieces = self.pieces[KNIGHT] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = KNIGHT_ATTACKS[from];
            score += MOBILITY_KNIGHT[count!(attacks & safe)];
        }

        // bishop mobility
        // - ignore friendly queens
        // - ignore enemy queens and rooks
        let mut occ = (boys | opps) ^ (self.pieces[KING] & opps) ^ self.pieces[QUEEN] ^ (rooks & opps);
        pieces = self.pieces[BISHOP] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ);
            score += MOBILITY_BISHOP[count!(attacks & safe)];
        }

        // rook mobility
        // - ingore friendly rooks and queens
        // - ignore enemy queens
        occ ^= rooks;
        pieces = rooks & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = rook_attacks(from, occ);
            score += MOBILITY_ROOK[count!(attacks & safe)];
        }

        score
    }
}