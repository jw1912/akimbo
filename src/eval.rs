use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

macro_rules! count {($bb:expr) => {($bb).count_ones() as i16}}
macro_rules! lsb {($x:expr) => {($x).trailing_zeros() as usize}}
macro_rules! pull_lsb {($idx:expr, $x:expr) => {$idx = lsb!($x); $x &= $x - 1}}

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
        (PAWN..=QUEEN).for_each(|i| score += self.material[i] * LAZY_MATERIAL[i]);

        // taper eval
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    pub fn eval(&self) -> i16 {
        let mut score: S = S(0, 0);

        // material scores
        (PAWN..=QUEEN).for_each(|i| score += self.material[i] * MATERIAL[i]);

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

        // pawn shield
        score += (count!(wp & wk_sqs) - count!(bp & bk_sqs)) * PAWN_SHIELD;

        // pawn pst
        let mut p: u64 = wp;
        while p > 0 {
            score += PAWN_PST[PST_IDX[56 ^ lsb!(p)] as usize];
            p &= p - 1;
        }
        p = bp;
        while p > 0 {
            score += -1 * PAWN_PST[PST_IDX[lsb!(p)] as usize];
            p &= p - 1;
        }

        // passed pawns
        score += (count!(wp & !bspans(bp | bp_att)) - count!(bp & !wspans(wp | wp_att))) * PAWN_PASSED;

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
        score += -danger * KING_LINEAR;
        score += -danger.pow(2) * KING_QUADRATIC;

        SIDE_FACTOR[c] * score
    }
}