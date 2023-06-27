use super::util::*;

macro_rules! bitloop {($bb:expr, $sq:ident, $func:expr) => {
    while $bb > 0 {
        let $sq = $bb.trailing_zeros() as u8;
        $bb &= $bb - 1;
        $func;
    }
}}

#[derive(Clone, Copy, Default)]
pub struct Position {
    bb: [u64; 8],
    pub c: bool,
    pub halfm: u8,
    pub enp_sq: u8,
    rights: u8,
    pub check: bool,
    hash: u64,
    pub phase: i32,
    pst: S,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub flag: u8,
    pub pc: u8,
}

#[derive(Clone, Copy)]
pub struct MoveList {
    pub list: [Move; 252],
    pub len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self { list: [Move::default(); 252], len: 0 }
    }
}

impl MoveList {
    fn push(&mut self, from: u8, to: u8, flag: u8, mpc: usize) {
        self.list[self.len] = Move { from, to, flag, pc: mpc as u8 };
        self.len += 1;
    }

    pub fn pick(&mut self, scores: &mut [i32; 252]) -> Option<(Move, i32)> {
        if self.len == 0 { return None }
        let (mut idx, mut best) = (0, i32::MIN);
        for (i, &score) in scores.iter().enumerate().take(self.len) {
            if score > best {
                best = score;
                idx = i;
            }
        }
        self.len -= 1;
        scores.swap(idx, self.len);
        self.list.swap(idx, self.len);
        Some((self.list[self.len], best))
    }
}

impl Position {
    // SEARCH STUFF

    pub fn hash(&self) -> u64 {
        let mut hash = self.hash;
        if self.enp_sq > 0 { hash ^= ZVALS.enp[self.enp_sq as usize & 7] }
        hash ^ ZVALS.cr[usize::from(self.rights)] ^ ZVALS.c[usize::from(self.c)]
    }

    pub fn draw(&self) -> bool {
        let (ph, b) = (self.phase, self.bb[Piece::BISHOP]);
        if self.halfm >= 100 { return true }
        ph <= 2 && self.bb[Piece::PAWN] == 0 && ((ph != 2) // no pawns left, phase <= 2
            || (b & self.bb[Side::WHITE] != b && b & self.bb[Side::BLACK] != b // one bishop each
                && (b & 0x55AA55AA55AA55AA == b || b & 0xAA55AA55AA55AA55 == b))) // same colour bishops
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[Piece::KING] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.sq_attacked(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

     // EVALUATION

    pub fn eval(&self) -> i32 {
        let (mut s, p) = (self.pst, 24.min(self.phase));

        // passed pawn eval
        let pawns = self.bb[Piece::PAWN];
        let (wp, bp) = (pawns & self.bb[Side::WHITE], pawns & self.bb[Side::BLACK]);
        let mut passed = Self::passers(wp, bp);
        bitloop!(passed, sq, s += Eval::PASSER[sq as usize / 8]);
        passed = Self::passers(bp.swap_bytes(), wp.swap_bytes());
        bitloop!(passed, sq, s -= Eval::PASSER[sq as usize / 8]);

        Eval::SIDE[usize::from(self.c)] * (p * s.0 + (24 - p) * s.1) / 24
    }

    fn passers(boys: u64, mut opps: u64) -> u64 {
        opps >>= 8;
        opps |= opps >> 8;
        opps |= opps >> 16;
        opps |= opps >> 32;
        opps |= (opps & !File::A) >> 1 | (opps & !File::H) << 1;
        boys & !opps
    }

    // MAKE MOVE

    fn toggle(&mut self, c: usize, pc: usize, bit: u64) {
        self.bb[pc] ^= bit;
        self.bb[c] ^= bit;
    }

    fn sq_attacked(&self, sq: usize, side: usize, occ: u64) -> bool {
        ( (Attacks::KNIGHT[sq] & self.bb[Piece::KNIGHT])
        | (Attacks::KING  [sq] & self.bb[Piece::KING  ])
        | (Attacks::PAWN  [side][sq] & self.bb[Piece::PAWN  ])
        | (Attacks::rook  (sq, occ) & (self.bb[Piece::ROOK  ] | self.bb[Piece::QUEEN]))
        | (Attacks::bishop(sq, occ) & (self.bb[Piece::BISHOP] | self.bb[Piece::QUEEN]))
        ) & self.bb[side ^ 1] > 0
    }

    pub fn get_pc(&self, bit: u64) -> usize {
        self.bb.iter().skip(2).position(|pc_bb| bit & pc_bb > 0).unwrap_or(usize::MAX - 1).wrapping_add(2)
    }

    pub fn make(&mut self, mov: Move) -> bool {
        let (from_bb, to_bb, moved) = (1 << mov.from, 1 << mov.to, usize::from(mov.pc));
        let (to, from) = (usize::from(mov.to), usize::from(mov.from));
        let captured = if mov.flag & Flag::CAP == 0 { Piece::EMPTY } else { self.get_pc(to_bb) };
        let side = usize::from(self.c);

        // update state
        self.rights &= CASTLE_MASK[to] & CASTLE_MASK[from];
        self.halfm = u8::from(moved > Piece::PAWN && mov.flag != Flag::CAP) * (self.halfm + 1);
        self.enp_sq = 0;
        self.c = !self.c;

        // move piece
        self.toggle(side, moved, from_bb ^ to_bb);
        self.hash ^= ZVALS.pcs[side][moved][from] ^ ZVALS.pcs[side][moved][to];
        self.pst += PST[side][moved][to];
        self.pst -= PST[side][moved][from];

        // captures
        if captured != Piece::EMPTY {
            let opp = side ^ 1;
            self.toggle(opp, captured, to_bb);
            self.hash ^= ZVALS.pcs[opp][captured][to];
            self.pst -= PST[opp][captured][to];
            self.phase -= Eval::PHASE[captured];
        }

        // more complex moves
        match mov.flag {
            Flag::DBL => self.enp_sq = mov.to ^ 8,
            Flag::KS | Flag::QS => {
                let (bits, rfr, rto) = ROOK_MOVES[usize::from(mov.flag == Flag::KS)][side];
                self.toggle(side, Piece::ROOK, bits);
                self.hash ^= ZVALS.pcs[side][Piece::ROOK][rfr] ^ ZVALS.pcs[side][Piece::ROOK][rto];
                self.pst -= PST[side][Piece::ROOK][rfr];
                self.pst += PST[side][Piece::ROOK][rto];
            },
            Flag::ENP => {
                let pawn_sq = to ^ 8;
                self.toggle(side ^ 1, Piece::PAWN, 1 << pawn_sq);
                self.hash ^= ZVALS.pcs[side ^ 1][Piece::PAWN][pawn_sq];
                self.pst -= PST[side ^ 1][Piece::PAWN][pawn_sq];
            },
            Flag::PROMO.. => {
                let promo = usize::from((mov.flag & 3) + 3);
                self.bb[Piece::PAWN] ^= to_bb;
                self.bb[promo] ^= to_bb;
                self.hash ^= ZVALS.pcs[side][Piece::PAWN][to] ^ ZVALS.pcs[side][promo][to];
                self.pst -= PST[side][Piece::PAWN][to];
                self.pst += PST[side][promo][to];
                self.phase += Eval::PHASE[promo];
            }
            _ => {}
        }

        // validating move
        let kidx = (self.bb[Piece::KING] & self.bb[side]).trailing_zeros() as usize;
        self.sq_attacked(kidx, side, self.bb[0] | self.bb[1])
    }

    // MOVEGEN

    pub fn movegen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::default();
        let (side, occ) = (usize::from(self.c), self.bb[0] | self.bb[1]);
        let (boys, opps) = (self.bb[side], self.bb[side ^ 1]);
        let pawns = self.bb[Piece::PAWN] & boys;

        if QUIETS {
            let r = self.rights;
            if r & [Rights::WHITE, Rights::BLACK][side] > 0 && !self.in_check() {
                if self.c {
                    if self.can_castle::<{ Side::BLACK }, 0>(occ, 59) { moves.push(60, 58, Flag::QS, Piece::KING) }
                    if self.can_castle::<{ Side::BLACK }, 1>(occ, 61) { moves.push(60, 62, Flag::KS, Piece::KING) }
                } else {
                    if self.can_castle::<{ Side::WHITE }, 0>(occ,  3) { moves.push( 4,  2, Flag::QS, Piece::KING) }
                    if self.can_castle::<{ Side::WHITE }, 1>(occ,  5) { moves.push( 4,  6, Flag::KS, Piece::KING) }
                }
            }

            let empty = !occ;
            let mut dbl = shift(side, shift(side, empty & Rank::DBL[side]) & empty) & pawns;
            let mut push = shift(side, empty) & pawns;
            let mut promo = push & Rank::PEN[side];
            push &= !Rank::PEN[side];
            bitloop!(push, from, moves.push(from, idx_shift::<8>(side, from), Flag::QUIET, Piece::PAWN));
            bitloop!(promo, from,
                for flag in Flag::PROMO..=Flag::QPR {moves.push(from, idx_shift::<8>(side, from), flag, Piece::PAWN)}
            );
            bitloop!(dbl, from, moves.push(from, idx_shift::<16>(side, from), Flag::DBL, Piece::PAWN));
        }

        if self.enp_sq > 0 {
            let mut attackers = Attacks::PAWN[side ^ 1][self.enp_sq as usize] & pawns;
            bitloop!(attackers, from, moves.push(from, self.enp_sq, Flag::ENP, Piece::PAWN));
        }
        let (mut attackers, mut promo) = (pawns & !Rank::PEN[side], pawns & Rank::PEN[side]);
        bitloop!(attackers, from, {
            let mut attacks = Attacks::PAWN[side][from as usize] & opps;
            bitloop!(attacks, to, moves.push(from, to, Flag::CAP, Piece::PAWN));
        });
        bitloop!(promo, from, {
            let mut attacks = Attacks::PAWN[side][from as usize] & opps;
            bitloop!(attacks, to, for flag in Flag::NPC..=Flag::QPC { moves.push(from, to, flag, Piece::PAWN) });
        });

        for pc in Piece::KNIGHT..=Piece::KING {
            let mut attackers = boys & self.bb[pc];
            bitloop!(attackers, from, {
                let attacks = match pc {
                    Piece::KNIGHT => Attacks::KNIGHT[from as usize],
                    Piece::BISHOP => Attacks::bishop(from as usize, occ),
                    Piece::ROOK   => Attacks::rook  (from as usize, occ),
                    Piece::QUEEN  => Attacks::bishop(from as usize, occ) | Attacks::rook(from as usize, occ),
                    Piece::KING   => Attacks::KING  [from as usize],
                    _ => unreachable!(),
                };
                let (mut caps, mut quiets) = (attacks & opps, attacks & !occ);
                bitloop!(caps, to, moves.push(from, to, Flag::CAP, pc));
                if QUIETS { bitloop!(quiets, to, moves.push(from, to, Flag::QUIET, pc));}
            });
        }
        moves
    }

    fn can_castle<const SIDE: usize, const KS: usize>(&self, occ: u64, sq: usize) -> bool {
        self.rights & [[Rights::WQS, Rights::WKS], [Rights::BQS, Rights::BKS]][SIDE][KS] > 0
            && occ & [[0xE, 0x60], [0xE00000000000000, 0x6000000000000000]][SIDE][KS] == 0
            && !self.sq_attacked(sq, SIDE, occ)
    }

    // PARSING FEN

    pub fn from_fen(fen: &str) -> Self {
        let vec = fen.split_whitespace().collect::<Vec<&str>>();
        let p = vec[0].chars().collect::<Vec<char>>();

        let (mut pos, mut row, mut col) = (Self::default(), 7, 0);
        for ch in p {
            if ch == '/' {
                row -= 1;
                col = 0;
            } else if ('1'..='8').contains(&ch) {
                col += ch.to_string().parse().unwrap_or(0);
            } else if let Some(idx) = "PNBRQKpnbrqk".chars().position(|el| el == ch) {
                let side = usize::from(idx > 5);
                let (pc, sq) = (idx + 2 - 6 * side, 8 * row + col);
                pos.toggle(side, pc, 1 << sq);
                pos.hash ^= ZVALS.pcs[side][pc][sq as usize];
                pos.pst += PST[side][pc][sq as usize];
                pos.phase += Eval::PHASE[pc];
                col += 1;
            }
        }

        pos.c = vec[1] == "b";
        pos.enp_sq = if vec[3] == "-" {0} else {
            let chs: Vec<char> = vec[3].chars().collect();
            8 * chs[1].to_string().parse::<u8>().unwrap() + chs[0] as u8 - 105
        };
        pos.halfm = vec.get(4).unwrap_or(&"0").parse::<u8>().unwrap();
        pos.rights = vec[2].chars().fold(0, |cr, ch| cr | match ch {
            'Q' => Rights::WQS, 'K' => Rights::WKS, 'q' => Rights::BQS, 'k' => Rights::BKS, _ => 0
        });

        pos
    }
}

fn shift(side: usize,bb: u64) -> u64 {
    if side == Side::WHITE { bb >> 8 } else { bb << 8 }
}

fn idx_shift<const AMOUNT: u8>(side: usize, idx: u8) -> u8 {
    if side == Side::WHITE { idx + AMOUNT } else { idx - AMOUNT }
}

impl Move {
    pub fn from_short(m: u16, pos: &Position) -> Self {
        let from = ((m >> 6) & 63) as u8;
        Self { from, to: (m & 63) as u8, flag: (m >> 12) as u8, pc: pos.get_pc(1 << from) as u8 }
    }

    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {["n","b","r","q"][(self.flag & 0b11) as usize]} else {""};
        format!("{}{}{}", idx_to_sq(self.from), idx_to_sq(self.to), promo)
    }
}
