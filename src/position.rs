use crate::{
    attacks::Attacks,
    bitloop,
    consts::{Flag, Piece, Rank, Rights, Side, PHASE_VALS, SEE_VALS, ZobristVals},
    frc::Castling,
    moves::{Move, MoveList},
    network::{Accumulator, EvalTable, Network},
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
    pawnhash: u64,
    pub phase: i32,
}

impl Position {
    pub fn side(&self, side: usize) -> u64 {
        self.bb[side]
    }

    pub fn piece(&self, pc: usize) -> u64 {
        self.bb[pc]
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
            hash ^= ZobristVals::en_passant(self.enp_sq);
        }

        hash ^ ZobristVals::castling(self.rights) ^ ZobristVals::side(self.stm())
    }

    pub fn pawnhash(&self) -> u64 {
        self.pawnhash
    }

    fn ksq(&self, side: usize) -> u8 {
        (self.bb[side] & self.bb[Piece::KING]).trailing_zeros() as u8
    }

    fn toggle(&mut self, side: usize, pc: usize, sq: usize) {
        let bit = 1 << sq;

        // toggle bitboards
        self.bb[pc] ^= bit;
        self.bb[side] ^= bit;

        // update hash
        let hash_val = ZobristVals::piece(side, pc, sq);
        self.hash ^= hash_val;

        if pc == Piece::PAWN {
            self.pawnhash ^= hash_val;
        }
    }

    pub fn has_non_pk(&self, side: usize) -> bool {
        let occ = self.bb[Side::WHITE] | self.bb[Side::BLACK];
        let pk = self.bb[Piece::PAWN] | self.bb[Piece::KING];
        self.bb[side] & (occ ^ pk) > 0
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

    pub fn make(&mut self, mov: Move, castling: &Castling) -> bool {
        let side = self.stm();
        let moved = mov.moved_pc();
        let to = mov.to();
        let from = mov.from();
        let captured = if mov.is_capture() {
            self.get_pc(1 << to)
        } else {
            Piece::EMPTY
        };

        // update state
        self.rights &= castling.mask(to) & castling.mask(from);
        self.halfm += 1;
        self.enp_sq = 0;
        self.c = !self.c;
        if moved == Piece::PAWN || mov.is_capture() {
            self.halfm = 0;
        }

        // move piece
        self.toggle(side, moved, from);
        if mov.flag() < Flag::PROMO {
            self.toggle(side, moved, to);
        }

        // captures
        if captured != Piece::EMPTY {
            self.toggle(side ^ 1, captured, to);
            self.phase -= PHASE_VALS[captured];
        }

        // more complex moves
        match mov.flag() {
            Flag::DBL => self.enp_sq = mov.to() as u8 ^ 8,
            Flag::KS | Flag::QS => {
                let ks = usize::from(mov.flag() == Flag::KS);
                let sf = 56 * side;
                let rfr = sf + castling.rook_file(side, ks) as usize;
                let rto = sf + [3, 5][ks];
                self.toggle(side, Piece::ROOK, rfr);
                self.toggle(side, Piece::ROOK, rto);
            }
            Flag::ENP => self.toggle(side ^ 1, Piece::PAWN, to ^ 8),
            Flag::PROMO.. => {
                let promo = mov.promo_pc();
                self.phase += PHASE_VALS[promo];
                self.toggle(side, promo, to);
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

    pub fn eval(&self, cache: &mut EvalTable) -> i32 {
        let wksq = self.ksq(Side::WHITE);
        let bksq = self.ksq(Side::BLACK);

        let wbucket = Network::get_bucket::<0>(wksq);
        let bbucket = Network::get_bucket::<1>(bksq);

        let entry = &mut cache.table[wbucket][bbucket];

        let mut addf = [[0; 32]; 2];
        let mut subf = [[0; 32]; 2];

        let (adds, subs) = self.fill_diff(&entry.bbs, &mut addf, &mut subf);

        entry.white.update_multi(&addf[0][..adds], &subf[0][..subs]);
        entry.black.update_multi(&addf[1][..adds], &subf[1][..subs]);

        entry.bbs = self.bb;

        self.eval_from_accs(&entry.white, &entry.black)
    }

    fn eval_from_accs(&self, white: &Accumulator, black: &Accumulator) -> i32 {
        let cnt = (self.bb[0] ^ self.bb[1]).count_ones() as usize;
        let bucket = (cnt - 2) / 4;

        let eval = if self.stm() == Side::WHITE {
            Network::out(white, black, bucket)
        } else {
            Network::out(black, white, bucket)
        };

        self.scale(eval)
    }

    pub fn eval_from_scratch(&self) -> i32 {
        let mut table = EvalTable::default();
        self.eval(&mut table)
    }

    fn fill_diff(
        &self,
        bbs: &[u64; 8],
        add_feats: &mut [[u16; 32]; 2],
        sub_feats: &mut [[u16; 32]; 2],
    ) -> (usize, usize) {
        let mut adds = 0;
        let mut subs = 0;

        let wksq = self.ksq(0);
        let bksq = self.ksq(1);

        let wflip = if wksq % 8 > 3 { 7 } else { 0 };
        let bflip = if bksq % 8 > 3 { 7 } else { 0 } ^ 56;

        for side in [Side::WHITE, Side::BLACK] {
            let old_boys = bbs[side];
            let new_boys = self.bb[side];

            for (piece, &(mut old_bb)) in bbs[Piece::PAWN..=Piece::KING].iter().enumerate() {
                old_bb &= old_boys;
                let new_bb = self.bb[piece + 2] & new_boys;

                let wbase = Network::get_base_index::<0>(side, piece, wksq) as u16;
                let bbase = Network::get_base_index::<1>(side, piece, bksq) as u16;

                let mut add_diff = new_bb & !old_bb;
                bitloop!(|add_diff, sq| {
                    let sq = u16::from(sq);
                    add_feats[0][adds] = wbase + (sq ^ wflip);
                    add_feats[1][adds] = bbase + (sq ^ bflip);
                    adds += 1;
                });

                let mut sub_diff = old_bb & !new_bb;
                bitloop!(|sub_diff, sq| {
                    let sq = u16::from(sq);
                    sub_feats[0][subs] = wbase + (sq ^ wflip);
                    sub_feats[1][subs] = bbase + (sq ^ bflip);
                    subs += 1;
                });
            }
        }

        (adds, subs)
    }

    pub fn key_after(&self, mut curr: u64, mov: Move) -> u64 {
        let side = self.stm();
        let opp = side ^ 1;
        let mpc = mov.moved_pc();

        curr ^= ZobristVals::side(1);
        curr ^= ZobristVals::piece(side, mpc, mov.from());
        curr ^= ZobristVals::piece(side, mpc, mov.to());

        if mov.is_capture() {
            curr ^= ZobristVals::piece(opp, self.get_pc(mov.bb_to()), mov.to());
        }

        curr
    }

    fn scale(&self, eval: i32) -> i32 {
        let mut mat = self.bb[Piece::KNIGHT].count_ones() as i32 * SEE_VALS[Piece::KNIGHT]
            + self.bb[Piece::BISHOP].count_ones() as i32 * SEE_VALS[Piece::BISHOP]
            + self.bb[Piece::ROOK].count_ones() as i32 * SEE_VALS[Piece::ROOK]
            + self.bb[Piece::QUEEN].count_ones() as i32 * SEE_VALS[Piece::QUEEN];

        mat = 700 + mat / 32;

        eval * mat / 1024
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
                || (b & self.bb[Side::WHITE] != b
                    && b & self.bb[Side::BLACK] != b
                    && (b & 0x55AA55AA55AA55AA == b || b & 0xAA55AA55AA55AA55 == b)))
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[Piece::KING] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.sq_attacked(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

    pub fn threats(&self) -> u64 {
        let mut threats = 0;

        let occ = self.bb[Side::WHITE] | self.bb[Side::BLACK];

        let side = self.stm() ^ 1;
        let opps = self.bb[side];

        let queens = self.bb[Piece::QUEEN];

        let mut rooks = opps & (self.bb[Piece::ROOK] | queens);
        let mut bishops = opps & (self.bb[Piece::BISHOP] | queens);
        let mut knights = opps & self.bb[Piece::KNIGHT];
        let mut kings = opps & self.bb[Piece::KING];

        bitloop!(|rooks, sq| threats |= Attacks::rook(sq as usize, occ));
        bitloop!(|bishops, sq| threats |= Attacks::bishop(sq as usize, occ));
        bitloop!(|knights, sq| threats |= Attacks::knight(sq as usize));
        bitloop!(|kings, sq| threats |= Attacks::king(sq as usize));

        let pawns = opps & self.bb[Piece::PAWN];
        threats |= if side == Side::WHITE {
            Attacks::white_pawn_setwise(pawns)
        } else {
            Attacks::black_pawn_setwise(pawns)
        };

        threats
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

    pub fn movegen<const QUIETS: bool>(&self, castling: &Castling) -> MoveList {
        let mut moves = MoveList::ZEROED;
        let side = usize::from(self.c);
        let occ = self.bb[0] | self.bb[1];
        let boys = self.bb[side];
        let opps = self.bb[side ^ 1];
        let pawns = self.bb[Piece::PAWN] & boys;

        // special quiet moves
        if QUIETS {
            if self.rights & [Rights::WHITE, Rights::BLACK][side] > 0 && !self.in_check() {
                let kbb = self.bb[Piece::KING] & self.bb[side];
                let ksq = kbb.trailing_zeros() as u8;
                if self.c {
                    if self.can_castle(Rights::BQS, occ, kbb, 1 << 58, 1 << 59, castling) {
                        moves.push(ksq, 58, Flag::QS, Piece::KING);
                    }
                    if self.can_castle(Rights::BKS, occ, kbb, 1 << 62, 1 << 61, castling) {
                        moves.push(ksq, 62, Flag::KS, Piece::KING);
                    }
                } else {
                    if self.can_castle(Rights::WQS, occ, kbb, 1 << 2, 1 << 3, castling) {
                        moves.push(ksq, 2, Flag::QS, Piece::KING);
                    }
                    if self.can_castle(Rights::WKS, occ, kbb, 1 << 6, 1 << 5, castling) {
                        moves.push(ksq, 6, Flag::KS, Piece::KING);
                    }
                }
            }

            // pawn pushes
            let empty = !occ;
            let mut dbl = shift(side, shift(side, empty & Rank::DBL[side]) & empty) & pawns;
            let mut push = shift(side, empty) & pawns;
            let mut promo = push & Rank::PEN[side];
            push &= !Rank::PEN[side];

            bitloop!(|push, from| moves.push(
                from,
                idx_shift::<8>(side, from),
                Flag::QUIET,
                Piece::PAWN
            ));

            bitloop!(|promo, from| for flag in Flag::PROMO..=Flag::QPR {
                moves.push(from, idx_shift::<8>(side, from), flag, Piece::PAWN);
            });

            bitloop!(|dbl, from| moves.push(
                from,
                idx_shift::<16>(side, from),
                Flag::DBL,
                Piece::PAWN
            ));
        }

        if self.enp_sq > 0 {
            let mut attackers = Attacks::pawn(side ^ 1, self.enp_sq as usize) & pawns;
            bitloop!(|attackers, from| moves.push(from, self.enp_sq, Flag::ENP, Piece::PAWN));
        }

        let mut attackers = pawns & !Rank::PEN[side];
        let mut promo = pawns & Rank::PEN[side];

        bitloop!(|attackers, from| {
            let mut attacks = Attacks::pawn(side, from as usize) & opps;
            bitloop!(|attacks, to| moves.push(from, to, Flag::CAP, Piece::PAWN));
        });

        bitloop!(|promo, from| {
            let mut attacks = Attacks::pawn(side, from as usize) & opps;
            bitloop!(|attacks, to| for flag in Flag::NPC..=Flag::QPC {
                moves.push(from, to, flag, Piece::PAWN);
            });
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

    fn path(&self, side: usize, mut path: u64, occ: u64) -> bool {
        bitloop!(|path, idx| if self.sq_attacked(idx as usize, side, occ) {
            return false;
        });

        true
    }

    fn can_castle(
        &self,
        right: u8,
        occ: u64,
        kbb: u64,
        kto: u64,
        rto: u64,
        castling: &Castling,
    ) -> bool {
        let side = usize::from(self.c);
        let ks = usize::from([Rights::BKS, Rights::WKS].contains(&right));
        let bit = 1 << (56 * side + usize::from(castling.rook_file(side, ks)));
        self.rights & right > 0
            && (occ ^ bit) & (btwn(kbb, kto) ^ kto) == 0
            && (occ ^ kbb) & (btwn(bit, rto) ^ rto) == 0
            && self.path(side, btwn(kbb, kto), occ)
    }

    pub fn from_fen(fen: &str, castling: &mut Castling) -> Self {
        let vec = fen.split_whitespace().collect::<Vec<&str>>();
        let p = vec[0].chars().collect::<Vec<char>>();

        // board
        let mut pos = Self::default();
        let mut row = 7i16;
        let mut col = 0i16;

        for ch in p {
            if ch == '/' {
                row -= 1;
                col = 0;
            } else if ('1'..='8').contains(&ch) {
                col += ch.to_string().parse().unwrap_or(0);
            } else if let Some(idx) = "PNBRQKpnbrqk".chars().position(|el| el == ch) {
                let side = usize::from(idx > 5);
                let pc = idx + 2 - 6 * side;
                let sq = 8 * row + col;

                pos.toggle(side, pc, sq as usize);
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

        pos.rights = castling.parse(&pos, vec[2]);

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

fn btwn(bit1: u64, bit2: u64) -> u64 {
    let min = bit1.min(bit2);
    (bit1.max(bit2) - min) ^ min
}
