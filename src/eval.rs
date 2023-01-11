use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

macro_rules! count {($bb:expr) => {$bb.count_ones() as i16}}
macro_rules! lsb {($x:expr) => {$x.trailing_zeros() as usize}}
macro_rules! pull_lsb {($idx:expr, $x:expr) => {$idx = lsb!($x); $x &= $x - 1}}

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

        // pawn bitboards
        let mut wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let mut bp: u64 = self.pieces[PAWN] & self.sides[BLACK];
        score += (count!(wp & KING_ATTACKS[wk_idx]) - count!(bp & KING_ATTACKS[bk_idx])) * PAWN_SHIELD;

        // mobility
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);
        score += self.mobility(WHITE, bp_att);
        score += self.mobility(BLACK, wp_att);

        // pawn pst
        while wp > 0 {
            score += PAWN_PST[PST_IDX[56 ^ lsb!(wp)] as usize];
            wp &= wp - 1;
        }
        while bp > 0 {
            score += -1 * PAWN_PST[PST_IDX[lsb!(bp)] as usize];
            bp &= bp - 1;
        }

        // taper eval
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    fn mobility(&self, c: usize, opp_att: u64) -> S {
        let mut score: S = S(0, 0);
        let mut from: usize;
        let mut attacks: u64;
        let mut pieces: u64;

        // bitboards
        let boys: u64 = self.sides[c];
        let opps: u64 = self.sides[c ^ 1];
        let safe: u64 = !boys & !opp_att;

        // knight mobility
        pieces = self.pieces[KNIGHT] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = KNIGHT_ATTACKS[from] & safe;
            score += count!(attacks) * MOBILITY_KNIGHT;
        }

        // bishop mobility
        // - ignore friendly queens
        // - ignore enemy queens and rooks
        let mut occ: u64 = (boys | opps) ^ (self.pieces[KING] & opps) ^ self.pieces[QUEEN] ^ (self.pieces[ROOK] & opps);
        pieces = self.pieces[BISHOP] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = bishop_attacks(from, occ) & safe;
            score += count!(attacks) * MOBILITY_BISHOP;
        }

        // rook mobility
        // - ignore friendly queens and rooks
        // - ignore enemy queens
        occ ^= self.pieces[ROOK];
        pieces = self.pieces[ROOK] & boys;
        while pieces > 0 {
            pull_lsb!(from, pieces);
            attacks = rook_attacks(from, occ) & safe;
            score += count!(attacks) * MOBILITY_ROOK;
        }

        SIDE_FACTOR[c] * score
    }
}