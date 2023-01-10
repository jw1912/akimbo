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
fn major_mobility<const PC: usize>(mut attackers: u64, occ: u64, friends: u64, danger: &mut i16, ksqs: u64, unprotected: u64) -> MajorMobility {
    let mut from: usize;
    let mut attacks: u64;
    let mut ret = MajorMobility::default();
    attackers &= friends;
    while attackers > 0 {
        from = attackers.trailing_zeros() as usize;
        attackers &= attackers - 1;
        attacks = match PC {
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
        let wb: u64 = self.pieces[BISHOP] & white;
        let bb: u64 = self.pieces[BISHOP] & black;
        let wr: u64 = self.pieces[ROOK] & white;
        let br: u64 = self.pieces[ROOK] & black;
        let wq: u64 = self.pieces[QUEEN] & white;
        let bq: u64 = self.pieces[QUEEN] & black;
        let wking_sqs: u64 = KING_ATTACKS[(self.pieces[KING] & white).trailing_zeros() as usize];
        let bking_sqs: u64 = KING_ATTACKS[(self.pieces[KING] & black).trailing_zeros() as usize];
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);

        // material scores
        let mut score: S = self.scores;

        // pawn progression bonus
        for i in 0..5 {
            score += (count!(wp & PAWN_RANKS[i + 1]) - count!(bp & PAWN_RANKS[4 - i])) * PROGRESS[i];
        }

        // major piece mobility
        let mut wking_danger: i16 = 0;
        let mut bking_danger: i16 = 0;
        let mut w_maj_mob: MajorMobility;
        let mut b_maj_mob: MajorMobility;

        // knight mobility
        w_maj_mob = major_mobility::<KNIGHT>(self.pieces[KNIGHT], occ, white, &mut bking_danger, bking_sqs, !bp_att);
        b_maj_mob = major_mobility::<KNIGHT>(self.pieces[KNIGHT], occ, black, &mut wking_danger, wking_sqs, !wp_att);
        score += (w_maj_mob.threat - b_maj_mob.threat) * MAJOR_THREAT[0];
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[0];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[0];

        // bishop mobility
        w_maj_mob = major_mobility::<BISHOP>(self.pieces[BISHOP], occ ^ wq, white, &mut bking_danger, bking_sqs, !bp_att);
        b_maj_mob = major_mobility::<BISHOP>(self.pieces[BISHOP], occ ^ bq, black, &mut wking_danger, wking_sqs, !wp_att);
        score += (w_maj_mob.threat - b_maj_mob.threat) * MAJOR_THREAT[1];
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[1];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[1];

        // rook mobility
        w_maj_mob = major_mobility::<ROOK>(self.pieces[ROOK], occ ^ wq ^ wr, white, &mut bking_danger, bking_sqs, !bp_att);
        b_maj_mob = major_mobility::<ROOK>(self.pieces[ROOK], occ ^ bq ^ br, black, &mut wking_danger, wking_sqs, !wp_att);
        score += (w_maj_mob.threat - b_maj_mob.threat) * MAJOR_THREAT[2];
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[2];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[2];

        // queen mobility
        w_maj_mob = major_mobility::<QUEEN>(self.pieces[QUEEN], occ ^ wq ^ wb ^ wr, white, &mut bking_danger, bking_sqs, !bp_att);
        b_maj_mob = major_mobility::<QUEEN>(self.pieces[QUEEN], occ ^ bq ^ bb ^ br, black, &mut wking_danger, wking_sqs, !wp_att);
        score += (w_maj_mob.threat - b_maj_mob.threat) * MAJOR_THREAT[3];
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[3];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[3];

        // king safety and pawn control
        score += (wking_danger - bking_danger) * KING_SAFETY;
        score += (count!(white & wp_att) - count!(black & bp_att)) * PAWN_DEFEND;
        score += (count!(black & wp_att) - count!(white & bp_att)) * PAWN_THREAT;
        score += (count!(wp & wking_sqs) - count!(bp & bking_sqs)) * PAWN_SHIELD;

        // passed pawns
        score += (count!(wp & !bspans(bp | bp_att)) - count!(bp & !wspans(wp | wp_att))) * PAWN_PASSED;

        // doubled and isolated pawns
        for file in 0..8 {
            score += (i16::from(RAILS[file] & wp == 0) * count!(FILES[file] & wp) - i16::from(RAILS[file] & bp == 0) * count!(FILES[file] & bp)) * PAWN_ISOLATED;
        }

        // bishop pair bonus
        score += (i16::from(wb & (wb - 1) > 0) - i16::from(bb & (bb - 1) > 0)) * BISHOP_PAIR;

        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}