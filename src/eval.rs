use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

macro_rules! count {($bb:expr) => {$bb.count_ones() as i16}}

#[derive(Default)]
struct MajorMobility {
    threats: i16,
    supports: i16,
    controls: i16,
}

#[inline]
fn major_mobility(pc: usize, mut attackers: u64, occ: u64, friends: u64, unprotected: u64, danger: &mut i16, ksqs: u64) -> MajorMobility {
    let mut from: usize;
    let mut attacks: u64;
    let mut ret = MajorMobility::default();
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
        ret.threats += count!(attacks & (occ & !friends)); // threats
        ret.supports += count!(attacks & friends); // supports
        ret.controls += count!(attacks & (!occ & unprotected)); // other safe mobility
        *danger += count!(attacks & ksqs);
    }
    ret
}

impl Position {
    #[inline]
    pub fn lazy_eval(&self) -> i16 {
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * self.scores.0 as i32 + (TPHASE - phase) * self.scores.1 as i32) / TPHASE) as i16
    }

    pub fn eval(&self) -> i16 {
        let mut score: S = self.scores;

        let white = self.sides[WHITE];
        let black = self.sides[BLACK];

        // pawn stuff
        let occ: u64 = self.sides[WHITE] | self.sides[BLACK];
        let wp: u64 = self.pieces[PAWN] & white;
        let bp: u64 = self.pieces[PAWN] & black;
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);

        // king danger stuff
        let mut wking_danger: i16 = 0;
        let mut bking_danger: i16 = 0;
        let wking_sqs: u64 = KING_ATTACKS[(self.pieces[KING] & white).trailing_zeros() as usize];
        let bking_sqs: u64 = KING_ATTACKS[(self.pieces[KING] & black).trailing_zeros() as usize];

        // major piece mobility
        for i in 0..4 {
            let w_maj_mob: MajorMobility = major_mobility(i + 1, self.pieces[i + 1], occ, white, !bp_att, &mut bking_danger, bking_sqs);
            let b_maj_mob: MajorMobility = major_mobility(i + 1, self.pieces[i + 1], occ, black, !wp_att, &mut wking_danger, wking_sqs);
            score += (w_maj_mob.threats - b_maj_mob.threats) * THREATS[i];
            score += (w_maj_mob.supports - b_maj_mob.supports) * SUPPORTS[i];
            score += (w_maj_mob.controls - b_maj_mob.controls) * CONTROLS[i];
        }

        score += (wking_danger - bking_danger) * KING_SAFETY;
        score += (count!(white & wp_att) - count!(black & bp_att)) * PAWN_SUPPORTS;
        score += (count!(black & wp_att) - count!(white & bp_att)) * PAWN_THREATS;
        score += (count!(wp & wking_sqs) - count!(bp & bking_sqs)) * PAWN_SHIELD;

        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}