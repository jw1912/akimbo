use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

macro_rules! count {($bb:expr) => {$bb.count_ones() as i16}}

#[inline(always)]
fn wspans(mut pwns: u64) -> u64 {
    pwns |= pwns << 8;
    pwns |= pwns << 16;
    pwns |= pwns << 32;
    pwns << 8
}

#[inline(always)]
fn bspans(mut pwns: u64) -> u64 {
    pwns |= pwns >> 8;
    pwns |= pwns >> 16;
    pwns |= pwns >> 32;
    pwns >> 8
}

#[derive(Default)]
struct MajorMobility {
    threat: i16,
    defend: i16,
    attack: i16,
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
        ret.threat += count!(attacks & (occ & !friends));
        ret.defend += count!(attacks & friends);
        ret.attack += count!(attacks & (!occ & unprotected));
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
        // useful bitboards
        let white = self.sides[WHITE];
        let black = self.sides[BLACK];
        let occ: u64 = self.sides[WHITE] | self.sides[BLACK];
        let wp: u64 = self.pieces[PAWN] & white;
        let bp: u64 = self.pieces[PAWN] & black;
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);
        let wking_sqs: u64 = KING_ATTACKS[(self.pieces[KING] & white).trailing_zeros() as usize];
        let bking_sqs: u64 = KING_ATTACKS[(self.pieces[KING] & black).trailing_zeros() as usize];

        // material scores
        let mut score: S = self.scores;

        // pawn progression bonus
        for i in 0..5 {
            score += (count!(wp & PAWN_RANKS[i + 1]) - count!(bp & PAWN_RANKS[4 - i])) * PROGRESS[i];
        }

        // major piece mobility
        let mut wking_danger: i16 = 0;
        let mut bking_danger: i16 = 0;
        for i in 0..4 {
            let w_maj_mob: MajorMobility = major_mobility(i + 1, self.pieces[i + 1], occ, white, !bp_att, &mut bking_danger, bking_sqs);
            let b_maj_mob: MajorMobility = major_mobility(i + 1, self.pieces[i + 1], occ, black, !wp_att, &mut wking_danger, wking_sqs);
            score += (w_maj_mob.threat - b_maj_mob.threat) * MAJOR_THREAT[i];
            score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[i];
            score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[i];
        }

        // king safety and pawn control
        score += (wking_danger - bking_danger) * KING_SAFETY;
        score += (count!(white & wp_att) - count!(black & bp_att)) * PAWN_DEFEND;
        score += (count!(black & wp_att) - count!(white & bp_att)) * PAWN_THREAT;
        score += (count!(wp & wking_sqs) - count!(bp & bking_sqs)) * PAWN_SHIELD;

        // passed pawns
        let mut fspans = bspans(bp);
        fspans |= (fspans & NOTH) >> 1 | (fspans & !FILE) << 1;
        let passers: i16 = count!(wp & !fspans);
        fspans = wspans(wp);
        fspans |= (fspans & NOTH) >> 1 | (fspans & !FILE) << 1;
        score += (passers - count!(bp & !fspans)) * PAWN_PASSED;

        // bishop pair bonus
        let wb: u64 = self.pieces[BISHOP] & white;
        let bb: u64 = self.pieces[BISHOP] & black;
        score += (i16::from(wb & (wb - 1) > 0) - i16::from(bb & (bb - 1) > 0)) * BISHOP_PAIR;

        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}