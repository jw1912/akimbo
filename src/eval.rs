use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

macro_rules! count {($bb:expr) => {$bb.count_ones() as i16}}
macro_rules! pull_lsb {($idx:expr, $x:expr) => {$idx = $x.trailing_zeros() as usize; $x &= $x - 1}}

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

        // pawn progression
        let wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let bp: u64 = self.pieces[PAWN] & self.sides[BLACK];
        (0..5).for_each(|i| score += (count!(wp & RANKS[i + 1]) - count!(bp & RANKS[4 - i])) * PROGRESS[i]);

        // king position
        let wk_idx: usize = (self.pieces[KING] & self.sides[WHITE]).trailing_zeros() as usize;
        let bk_idx: usize = (self.pieces[KING] & self.sides[BLACK]).trailing_zeros() as usize;
        score += KING_RANKS[wk_idx / 8];
        score += -1 * KING_RANKS[7 - bk_idx / 8];

        // pawn shield
        score += (count!(wp & KING_ATTACKS[wk_idx]) - count!(bp & KING_ATTACKS[bk_idx])) * PAWN_SHIELD;

        // mobility
        score += self.mobility(WHITE);
        score += self.mobility(BLACK);

        // taper eval
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    fn mobility(&self, c: usize) -> S {
        let mut score: S = S(0, 0);
        let mut from: usize;
        let mut attacks: u64;

        // sides
        let boys: u64 = self.sides[c];
        let opps: u64 = self.sides[c ^ 1];

        // knight mobility
        let mut n: u64 = self.pieces[KNIGHT] & boys;
        while n > 0 {
            pull_lsb!(from, n);
            attacks = KNIGHT_ATTACKS[from];
            score += count!(attacks &  boys) * MAJOR_DEFEND[0];
            score += count!(attacks & !boys) * MAJOR_ATTACK[0];
        }

        // bishop mobility
        // - ignore friendly queens
        // - ignore enemy queens and rooks
        let mut occ: u64 = (boys | opps) ^ (self.pieces[KING] & opps) ^ self.pieces[QUEEN] ^ (self.pieces[ROOK] & opps);
        let mut b: u64 = self.pieces[BISHOP] & boys;
        while b > 0 {
            pull_lsb!(from, b);
            attacks = bishop_attacks(from, occ);
            score += count!(attacks &  boys) * MAJOR_DEFEND[1];
            score += count!(attacks & !boys) * MAJOR_ATTACK[1];
        }

        // rook mobility
        // - ignore friendly queens and rooks
        // - ignore enemy queens
        occ ^= self.pieces[ROOK];
        let mut r: u64 = self.pieces[ROOK] & boys;
        while r > 0 {
            pull_lsb!(from, r);
            attacks = rook_attacks(from, occ);
            score += count!(attacks &  boys) * MAJOR_DEFEND[2];
            score += count!(attacks & !boys) * MAJOR_ATTACK[2];
        }

        SIDE_FACTOR[c] * score
    }
}