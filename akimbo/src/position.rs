use crate::{
    attacks::Attacks,
    bitloop,
    consts::{
        Flag, Piece, Rank, Rights, Side, CASTLE_MASK, PHASE_VALS, ROOK_MOVES, SEE_VALS, SPANS,
        ZVALS,
    },
    moves::{Move, MoveList},
    network::{Accumulator, Network},
};

#[derive(Clone, Copy, Default)]
pub struct Position {
    bb: [u64; 8],
    c: bool,
    halfm: u8,
    enp_sq: u8,
    rights: u8,
    pub check: bool,
    hash: u64,
    pub phase: i32,
    acc: [Accumulator; 2],
}

impl Position {
    pub fn side(&self, side: usize) -> u64 {
        self.bb[side]
    }

    pub fn halfm(&self) -> usize {
        usize::from(self.halfm)
    }

    pub fn stm(&self) -> usize {
        usize::from(self.c)
    }

    pub fn hash(&self) -> u64 {
        let mut hash = self.hash;

        if self.enp_sq > 0 {
            hash ^= ZVALS.enp[self.enp_sq as usize & 7];
        }

        hash ^ ZVALS.cr[usize::from(self.rights)] ^ ZVALS.c[usize::from(self.c)]
    }

    fn toggle<const ADD: bool>(&mut self, side: usize, pc: usize, sq: usize) {
        let bit = 1 << sq;

        // toggle bitboards
        self.bb[pc] ^= bit;
        self.bb[side] ^= bit;

        // update hash
        self.hash ^= ZVALS.pcs[side][pc][sq];

        // update accumulators
        let start = 384 * side + 64 * pc + sq - 128;
        self.acc[0].update::<ADD>(start);

        let start = 384 * (side ^ 1) + 64 * pc + (sq ^ 56) - 128;
        self.acc[1].update::<ADD>(start);
    }

    fn sq_attacked(&self, sq: usize, side: usize, occ: u64) -> bool {
        ((Attacks::knight(sq) & self.bb[Piece::KNIGHT])
            | (Attacks::king(sq) & self.bb[Piece::KING])
            | (Attacks::pawn(side, sq) & self.bb[Piece::PAWN])
            | (Attacks::rook(sq, occ) & (self.bb[Piece::ROOK] | self.bb[Piece::QUEEN]))
            | (Attacks::bishop(sq, occ) & (self.bb[Piece::BISHOP] | self.bb[Piece::QUEEN])))
            & self.bb[side ^ 1]
            > 0
    }

    pub fn get_pc(&self, bit: u64) -> usize {
        self.bb
            .iter()
            .skip(2)
            .position(|pc_bb| bit & pc_bb > 0)
            .unwrap_or(usize::MAX - 1)
            .wrapping_add(2)
    }

    pub fn make(&mut self, mov: Move) -> bool {
        let side = usize::from(self.c);
        let moved = mov.moved_pc();
        let to = mov.to();
        let from = mov.from();
        let captured = if mov.is_capture() {
            self.get_pc(1 << to)
        } else {
            Piece::EMPTY
        };

        // update state
        self.rights &= CASTLE_MASK[to] & CASTLE_MASK[from];
        self.halfm += 1;
        self.enp_sq = 0;
        self.c = !self.c;
        if moved == Piece::PAWN || mov.is_capture() {
            self.halfm = 0;
        }

        // move piece
        self.toggle::<false>(side, moved, from);
        self.toggle::<true>(side, moved, to);

        // captures
        if captured != Piece::EMPTY {
            self.toggle::<false>(side ^ 1, captured, to);
            self.phase -= PHASE_VALS[captured];
        }

        // more complex moves
        match mov.flag() {
            Flag::DBL => self.enp_sq = mov.to() as u8 ^ 8,
            Flag::KS | Flag::QS => {
                let (rfr, rto) = ROOK_MOVES[usize::from(mov.flag() == Flag::KS)][side];
                self.toggle::<false>(side, Piece::ROOK, rfr);
                self.toggle::<true>(side, Piece::ROOK, rto);
            }
            Flag::ENP => self.toggle::<false>(side ^ 1, Piece::PAWN, to ^ 8),
            Flag::PROMO.. => {
                let promo = mov.promo_pc();
                self.phase += PHASE_VALS[promo];
                self.toggle::<false>(side, Piece::PAWN, to);
                self.toggle::<true>(side, promo, to);
            }
            _ => {}
        }

        // validating move
        let kidx = (self.bb[Piece::KING] & self.bb[side]).trailing_zeros() as usize;
        self.sq_attacked(kidx, side, self.bb[0] | self.bb[1])
    }

    pub fn make_null(&mut self) {
        self.c = !self.c;
        self.enp_sq = 0;
    }

    pub fn eval(&self) -> i32 {
        let boys = &self.acc[usize::from(self.c)];
        let opps = &self.acc[usize::from(!self.c)];
        Network::out(boys, opps)
    }

    pub fn draw(&self) -> bool {
        if self.halfm >= 100 {
            return true;
        }

        let ph = self.phase;
        let b = self.bb[Piece::BISHOP];
        ph <= 2
            && self.bb[Piece::PAWN] == 0
            && ((ph != 2)
            || (b & self.bb[Side::WHITE] != b && b & self.bb[Side::BLACK] != b
                && (b & 0x55AA55AA55AA55AA == b || b & 0xAA55AA55AA55AA55 == b)))
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[Piece::KING] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.sq_attacked(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

    pub fn is_passer(&self, sq: usize, side: usize) -> bool {
        SPANS[side][sq] & self.bb[Piece::PAWN] & self.bb[side ^ 1] == 0
    }

    fn gain(&self, mov: Move) -> i32 {
        if mov.is_en_passant() {
            return SEE_VALS[Piece::PAWN];
        }
        let mut score = SEE_VALS[self.get_pc(mov.bb_to())];
        if mov.is_promo() {
            score += SEE_VALS[mov.promo_pc()] - SEE_VALS[Piece::PAWN];
        }
        score
    }

    pub fn see(&self, mov: Move, threshold: i32) -> bool {
        let sq = mov.to();
        let mut next = if mov.is_promo() {
            mov.promo_pc()
        } else {
            mov.moved_pc()
        };
        let mut score = self.gain(mov) - threshold - SEE_VALS[next];

        if score >= 0 {
            return true;
        }

        let mut occ = (self.bb[Side::WHITE] | self.bb[Side::BLACK]) ^ mov.bb_from() ^ (1 << sq);
        if mov.is_en_passant() {
            occ ^= 1 << (sq ^ 8);
        }

        let bishops = self.bb[Piece::BISHOP] | self.bb[Piece::QUEEN];
        let rooks = self.bb[Piece::ROOK] | self.bb[Piece::QUEEN];
        let mut us = usize::from(!self.c);
        let mut attackers = (Attacks::knight(sq) & self.bb[Piece::KNIGHT])
            | (Attacks::king(sq) & self.bb[Piece::KING])
            | (Attacks::pawn(Side::WHITE, sq) & self.bb[Piece::PAWN] & self.bb[Side::BLACK])
            | (Attacks::pawn(Side::BLACK, sq) & self.bb[Piece::PAWN] & self.bb[Side::WHITE])
            | (Attacks::rook(sq, occ) & rooks)
            | (Attacks::bishop(sq, occ) & bishops);

        loop {
            let our_attackers = attackers & self.bb[us];
            if our_attackers == 0 {
                break;
            }

            for pc in Piece::PAWN..=Piece::KING {
                let board = our_attackers & self.bb[pc];
                if board > 0 {
                    occ ^= board & board.wrapping_neg();
                    next = pc;
                    break;
                }
            }

            if [Piece::PAWN, Piece::BISHOP, Piece::QUEEN].contains(&next) {
                attackers |= Attacks::bishop(sq, occ) & bishops;
            }
            if [Piece::ROOK, Piece::QUEEN].contains(&next) {
                attackers |= Attacks::rook(sq, occ) & rooks;
            }

            attackers &= occ;
            score = -score - 1 - SEE_VALS[next];
            us ^= 1;

            if score >= 0 {
                if next == Piece::KING && attackers & self.bb[us] > 0 {
                    us ^= 1;
                }
                break;
            }
        }

        self.c != (us == 1)
    }

    pub fn movegen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::ZEROED;
        let side = usize::from(self.c);
        let occ = self.bb[0] | self.bb[1];
        let boys = self.bb[side];
        let opps = self.bb[side ^ 1];
        let pawns = self.bb[Piece::PAWN] & boys;

        // special quiet moves
        if QUIETS {
            if self.rights & [Rights::WHITE, Rights::BLACK][side] > 0 && !self.in_check() {
                if self.c {
                    if self.can_castle::<{ Side::BLACK }, 0>(occ, 59) {
                        moves.push(60, 58, Flag::QS, Piece::KING);
                    }
                    if self.can_castle::<{ Side::BLACK }, 1>(occ, 61) {
                        moves.push(60, 62, Flag::KS, Piece::KING);
                    }
                } else {
                    if self.can_castle::<{ Side::WHITE }, 0>(occ, 3) {
                        moves.push(4, 2, Flag::QS, Piece::KING);
                    }
                    if self.can_castle::<{ Side::WHITE }, 1>(occ, 5) {
                        moves.push(4, 6, Flag::KS, Piece::KING);
                    }
                }
            }

            // pawn pushes
            let empty = !occ;
            let mut dbl = shift(side, shift(side, empty & Rank::DBL[side]) & empty) & pawns;
            let mut push = shift(side, empty) & pawns;
            let mut promo = push & Rank::PEN[side];
            push &= !Rank::PEN[side];

            bitloop!(|push, from|
                moves.push(from, idx_shift::<8>(side, from), Flag::QUIET, Piece::PAWN)
            );

            bitloop!(|promo, from|
                for flag in Flag::PROMO..=Flag::QPR {
                    moves.push(from, idx_shift::<8>(side, from), flag, Piece::PAWN);
                }
            );

            bitloop!(|dbl, from|
                moves.push(from, idx_shift::<16>(side, from), Flag::DBL, Piece::PAWN)
            );
        }

        if self.enp_sq > 0 {
            let mut attackers = Attacks::pawn(side ^ 1, self.enp_sq as usize) & pawns;
            bitloop!(|attackers, from|
                moves.push(from, self.enp_sq, Flag::ENP, Piece::PAWN)
            );
        }

        let (mut attackers, mut promo) = (pawns & !Rank::PEN[side], pawns & Rank::PEN[side]);

        bitloop!(|attackers, from| {
            let mut attacks = Attacks::pawn(side, from as usize) & opps;
            bitloop!(|attacks, to| moves.push(from, to, Flag::CAP, Piece::PAWN));
        });

        bitloop!(|promo, from| {
            let mut attacks = Attacks::pawn(side, from as usize) & opps;
            bitloop!(|attacks, to|
                for flag in Flag::NPC..=Flag::QPC {
                    moves.push(from, to, flag, Piece::PAWN);
                }
            );
        });

        // non-pawn moves
        for pc in Piece::KNIGHT..=Piece::KING {
            let mut attackers = boys & self.bb[pc];
            bitloop!(|attackers, from| {
                let attacks = match pc {
                    Piece::KNIGHT => Attacks::knight(from as usize),
                    Piece::BISHOP => Attacks::bishop(from as usize, occ),
                    Piece::ROOK => Attacks::rook(from as usize, occ),
                    Piece::QUEEN => Attacks::queen(from as usize, occ),
                    Piece::KING => Attacks::king(from as usize),
                    _ => unreachable!(),
                };

                let mut caps = attacks & opps;
                bitloop!(|caps, to| moves.push(from, to, Flag::CAP, pc));

                if QUIETS {
                    let mut quiets = attacks & !occ;
                    bitloop!(|quiets, to| moves.push(from, to, Flag::QUIET, pc));
                }
            });
        }
        moves
    }

    fn can_castle<const SIDE: usize, const KS: usize>(&self, occ: u64, sq: usize) -> bool {
        self.rights & [[Rights::WQS, Rights::WKS], [Rights::BQS, Rights::BKS]][SIDE][KS] > 0
            && occ & [[0xE, 0x60], [0xE00000000000000, 0x6000000000000000]][SIDE][KS] == 0
            && !self.sq_attacked(sq, SIDE, occ)
    }

    pub fn from_fen(fen: &str) -> Self {
        let vec = fen.split_whitespace().collect::<Vec<&str>>();
        let p = vec[0].chars().collect::<Vec<char>>();

        // board
        let mut pos = Self::default();
        let (mut row, mut col) = (7i16, 0i16);

        for ch in p {
            if ch == '/' {
                row -= 1;
                col = 0;
            } else if ('1'..='8').contains(&ch) {
                col += ch.to_string().parse().unwrap_or(0);
            } else if let Some(idx) = "PNBRQKpnbrqk".chars().position(|el| el == ch) {
                let side = usize::from(idx > 5);
                let (pc, sq) = (idx + 2 - 6 * side, 8 * row + col);
                pos.toggle::<true>(side, pc, sq as usize);
                pos.phase += PHASE_VALS[pc];
                col += 1;
            }
        }

        // state
        pos.c = vec[1] == "b";
        pos.enp_sq = if vec[3] == "-" {
            0
        } else {
            let chs: Vec<char> = vec[3].chars().collect();
            8 * chs[1].to_string().parse::<u8>().unwrap() + chs[0] as u8 - 105
        };
        pos.halfm = vec.get(4).unwrap_or(&"0").parse::<u8>().unwrap();
        pos.rights = vec[2].chars().fold(0, |cr, ch| {
            cr | match ch {
                'Q' => Rights::WQS,
                'K' => Rights::WKS,
                'q' => Rights::BQS,
                'k' => Rights::BKS,
                _ => 0,
            }
        });

        pos
    }
}

fn shift(side: usize, bb: u64) -> u64 {
    if side == Side::WHITE {
        bb >> 8
    } else {
        bb << 8
    }
}

fn idx_shift<const AMOUNT: u8>(side: usize, idx: u8) -> u8 {
    if side == Side::WHITE {
        idx + AMOUNT
    } else {
        idx - AMOUNT
    }
}
