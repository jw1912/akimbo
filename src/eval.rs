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
    defend: i16,
    attack: i16,
}

#[inline]
fn major_mobility<const PC: usize>(mut attackers: u64, occ: u64, friends: u64) -> MajorMobility {
    let mut from: usize;
    let mut attacks: u64;
    let mut ret = MajorMobility::default();
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
        ret.defend += count!(attacks & friends);
        ret.attack += count!(attacks & !friends);
    }
    ret
}

impl Position {
    #[inline]
    pub fn lazy_eval(&self) -> i16 {
        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        let mut score: S = self.material[PAWN] * LAZY_MATERIAL[PAWN];
        score += self.material[KNIGHT] * LAZY_MATERIAL[KNIGHT];
        score += self.material[BISHOP] * LAZY_MATERIAL[BISHOP];
        score += self.material[ROOK  ] * LAZY_MATERIAL[ROOK  ];
        score += self.material[QUEEN ] * LAZY_MATERIAL[QUEEN ];
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }

    pub fn eval(&self) -> i16 {
        // useful bitboards
        let white: u64 = self.sides[WHITE];
        let black: u64 = self.sides[BLACK];
        let occ: u64 = self.sides[WHITE] | self.sides[BLACK];
        let wp: u64 = self.pieces[PAWN] & white;
        let bp: u64 = self.pieces[PAWN] & black;
        let wb: u64 = self.pieces[BISHOP] & white;
        let bb: u64 = self.pieces[BISHOP] & black;
        let wr: u64 = self.pieces[ROOK] & white;
        let br: u64 = self.pieces[ROOK] & black;
        let wq: u64 = self.pieces[QUEEN] & white;
        let bq: u64 = self.pieces[QUEEN] & black;
        let wk: u64 = self.pieces[KING] & white;
        let bk: u64 = self.pieces[KING] & black;
        let wk_idx: usize = wk.trailing_zeros() as usize;
        let bk_idx: usize = bk.trailing_zeros() as usize;
        let wking_sqs: u64 = KING_ATTACKS[wk_idx];
        let bking_sqs: u64 = KING_ATTACKS[bk_idx];
        let wp_att: u64 = ((wp & !FILE) << 7) | ((wp & NOTH) << 9);
        let bp_att: u64 = ((bp & !FILE) >> 9) | ((bp & NOTH) >> 7);

        // material scores
        let mut score: S = self.material[PAWN] * MATERIAL[PAWN];
        score += self.material[KNIGHT] * MATERIAL[KNIGHT];
        score += self.material[BISHOP] * MATERIAL[BISHOP];
        score += self.material[ROOK  ] * MATERIAL[ROOK  ];
        score += self.material[QUEEN ] * MATERIAL[QUEEN ];

        // pawn progression bonus
        for i in 0..5 {
            score += (count!(wp & PAWN_RANKS[i + 1]) - count!(bp & PAWN_RANKS[4 - i])) * PROGRESS[i];
        }

        // major piece mobility
        let mut w_maj_mob: MajorMobility;
        let mut b_maj_mob: MajorMobility;

        // knight mobility
        w_maj_mob = major_mobility::<KNIGHT>(self.pieces[KNIGHT] & white, occ, white);
        b_maj_mob = major_mobility::<KNIGHT>(self.pieces[KNIGHT] & black, occ, black);
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[0];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[0];

        // bishop mobility - ignore friendly queens
        w_maj_mob = major_mobility::<BISHOP>(wb, occ ^ wq ^ bk, white);
        b_maj_mob = major_mobility::<BISHOP>(bb, occ ^ bq ^ wk, black);
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[1];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[1];

        // rook mobility - ignore friendly queens and rooks
        w_maj_mob = major_mobility::<ROOK>(wr, occ ^ wq ^ wr ^ bk, white);
        b_maj_mob = major_mobility::<ROOK>(br, occ ^ bq ^ br ^ wk, black);
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[2];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[2];

        // queen mobility - ignore friendly queens, rooks and bishops
        w_maj_mob = major_mobility::<QUEEN>(wq, occ ^ wq ^ wb ^ wr ^ bk, white);
        b_maj_mob = major_mobility::<QUEEN>(bq, occ ^ bq ^ bb ^ br ^ wk, black);
        score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[3];
        score += (w_maj_mob.attack - b_maj_mob.attack) * MAJOR_ATTACK[3];

        // king safety and pawn control
        score += (count!(wp & wking_sqs) - count!(bp & bking_sqs)) * PAWN_SHIELD;
        score += KING_RANKS[wk_idx / 8];
        score += -1 * KING_RANKS[7 - bk_idx / 8];

        // passed pawns
        score += (count!(wp & !bspans(bp | bp_att)) - count!(bp & !wspans(wp | wp_att))) * PAWN_PASSED;

        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}