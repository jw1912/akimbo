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
const MATERIAL: [S; 5] = [S(84, 145), S(321, 259), S(343, 272), S(424, 512), S(931, 920)];
const PAWN_HT: [S; 24] = [
    S( 82, 121), S( 99, 115), S( 94,  91), S(110,  74),
    S(-31,  45), S(  3,  39), S( 24,  16), S( 40,   8),
    S(-39, -15), S(-17, -22), S(-18, -35), S( -4, -47),
    S(-40, -36), S(-22, -34), S(-16, -50), S( -8, -53),
    S(-34, -42), S( -7, -41), S(-22, -48), S(-16, -46),
    S(-41, -40), S(-13, -34), S(-23, -37), S(-30, -33),
];
const KING_QT: [S; 16] = [
    S(-63,  10), S(-30,  29), S(-29,  40), S(-36,  42),
    S(-49,   8), S(-35,  23), S(-43,  31), S(-50,  38),
    S( 21, -17), S( -5,   7), S(-35,  23), S(-48,  25),
    S( 29, -53), S( 38, -25), S( -7,  -8), S( 13, -28),
];
const MOBILITY_KNIGHT: [S; 9] = [
    S(-40, -79), S(-12, -59), S( -3, -33),
    S(  4, -14), S( 15,  -3), S( 19,  13),
    S( 24,  16), S( 27,  23), S( 39,  11),
];
const MOBILITY_BISHOP: [S; 14] = [
    S(-16, -81), S( -6, -55), S(  2, -29), S(  7, -12), S( 11,   0), S( 14,   9), S( 18,  17),
    S( 19,  16), S( 21,  24), S( 25,  22), S( 35,  23), S( 44,  21), S( 38,  30), S( 46,  24),
];
const PAWN_SHIELD: S = S(20, -4);

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

        score * SIDE[c]
    }
}