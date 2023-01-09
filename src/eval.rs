use super::{consts::*, movegen::{rook_attacks, bishop_attacks}, position::{Position, S}};

macro_rules! count {($bb:expr) => {$bb.count_ones() as i16}}

#[derive(Default)]
struct MajorMobility {
    threat: i16,
    defend: i16,
}

#[inline]
fn major_mobility(pc: usize, mut attackers: u64, occ: u64, friends: u64, danger: &mut i16, ksqs: u64) -> MajorMobility {
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
            // rooks don't block each other, rooks and bishops don't block queen, queen blocks nothing
            let (tw, tb): (u64, u64) = match i + 1 {
                ROOK => {
                    let qr: u64 = self.pieces[ROOK] ^ self.pieces[QUEEN];
                    (qr & white, qr & black)
                },
                QUEEN => {
                    let br: u64 = self.pieces[BISHOP] ^ self.pieces[ROOK] ^ self.pieces[QUEEN];
                    (br & white, br & black)
                },
                _ => (self.pieces[QUEEN] & white, self.pieces[QUEEN] & black)
            };
            let w_maj_mob: MajorMobility = major_mobility(i + 1, self.pieces[i + 1], occ ^ tw, white, &mut bking_danger, bking_sqs);
            let b_maj_mob: MajorMobility = major_mobility(i + 1, self.pieces[i + 1], occ ^ tb, black, &mut wking_danger, wking_sqs);
            score += (w_maj_mob.threat - b_maj_mob.threat) * MAJOR_THREAT[i];
            score += (w_maj_mob.defend - b_maj_mob.defend) * MAJOR_DEFEND[i];
        }

        // king safety and pawn control
        score += (wking_danger - bking_danger) * KING_SAFETY;
        score += (count!(wp & wking_sqs) - count!(bp & bking_sqs)) * PAWN_SHIELD;

        let phase: i32 = std::cmp::min(self.phase as i32, TPHASE);
        SIDE_FACTOR[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}