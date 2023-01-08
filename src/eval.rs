use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

#[inline]
pub fn major_mobility(pc: usize, mut attackers: u64, occ: u64, friends: u64, unprotected: u64) -> (i16, i16, i16) {
    let mut from: usize;
    let mut attacks: u64;
    let mut ret: (i16, i16, i16) = (0, 0, 0);
    attackers &= friends;
    while attackers > 0 {
        from = attackers.trailing_zeros() as usize;
        attackers &= attackers - 1;
        attacks = match pc {
            KNIGHT => KNIGHT_ATTACKS[from],
            ROOK => rook_attacks(from, occ),
            BISHOP => bishop_attacks(from, occ),
            QUEEN => rook_attacks(from, occ) | bishop_attacks(from, occ),
            _ => unimplemented!("only implement the four major pieces"),
        };
        ret.0 += (attacks & (occ & !friends)).count_ones() as i16; // threats
        ret.1 += (attacks & friends).count_ones() as i16; // supports
        ret.2 += (attacks & (!occ & unprotected)).count_ones() as i16; // other safe mobility
    }
    ret
}

impl Position {
    pub fn eval(&self) -> i16 {
        let mut score: S = self.scores;

        // mobility
        let occ: u64 = self.sides[WHITE] | self.sides[BLACK];
        let wp: u64 = self.pieces[PAWN] & self.sides[WHITE];
        let bp: u64 = self.pieces[PAWN] & self.sides[BLACK];
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);
        for i in 0..4 {
            let (w_thr, w_sup, w_oth): (i16, i16, i16) = major_mobility(i + 1, self.pieces[i + 1], occ, self.sides[WHITE], !bp_att);
            let (b_thr, b_sup, b_oth): (i16, i16, i16) = major_mobility(i + 1, self.pieces[i + 1], occ, self.sides[BLACK], !wp_att);
            score += (w_thr - b_thr) * THREATS[i];
            score += (w_sup - b_sup) * SUPPORTS[i];
            score += (w_oth - b_oth) * CONTROLS[i];
        }

        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}