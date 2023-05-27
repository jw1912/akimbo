use super::util::{Attacks, Flag, PHASE_VALS, Piece, PST, Rank, Rights, S, Side, SIDE, ZVALS};
use std::sync::atomic::{AtomicU8, AtomicBool, Ordering::Relaxed};

macro_rules! bitloop {($bb:expr, $sq:ident, $func:expr) => {
    while $bb > 0 {
        let $sq = $bb.trailing_zeros() as u8;
        $bb &= $bb - 1;
        $func;
    }
}}

#[allow(clippy::declare_interior_mutable_const)]
const INIT: AtomicU8 = AtomicU8::new(0);
static CHESS960: AtomicBool = AtomicBool::new(false);
static CASTLE_MASK: [AtomicU8; 64] = [INIT; 64];
static ROOK_FILES: [[AtomicU8; 2]; 2] = [[INIT; 2], [INIT; 2]];

#[derive(Clone, Copy, Default)]
pub struct Position {
    bb: [u64; 8],
    pub c: bool,
    pub halfm: u8,
    pub enp_sq: u8,
    rights: u8,
    pub check: bool,
    hash: u64,
    pub phase: i16,
    pub nulls: i16,
    pst: S,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub flag: u8,
    pub pc: u8,
}

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

    pub fn pick(&mut self, scores: &mut [i16; 252]) -> Option<(Move, i16)> {
        if self.len == 0 { return None }
        let (mut idx, mut best) = (0, i16::MIN);
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
    pub fn hash(&self) -> u64 {
        let mut hash = self.hash;
        if self.enp_sq > 0 { hash ^= ZVALS.enp[self.enp_sq as usize & 7] }
        hash ^ ZVALS.cr[usize::from(self.rights)] ^ ZVALS.c[usize::from(self.c)]
    }

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
        usize::from(
            (self.bb[Piece::KNIGHT] | self.bb[Piece::ROOK] | self.bb[Piece::KING]) & bit > 0
        ) | (2 * usize::from(
            (self.bb[Piece::KNIGHT] | self.bb[Piece::PAWN] | self.bb[Piece::QUEEN] | self.bb[Piece::KING]) & bit > 0)
        ) | (4 * usize::from(
            (self.bb[Piece::BISHOP] | self.bb[Piece::ROOK] | self.bb[Piece::QUEEN] | self.bb[Piece::KING]) & bit > 0)
        )
    }

    pub fn make(&mut self, mov: Move) -> bool {
        let (from_bb, to_bb, moved) = (1 << mov.from, 1 << mov.to, usize::from(mov.pc));
        let (to, from) = (usize::from(mov.to), usize::from(mov.from));
        let captured = if mov.flag & Flag::CAP == 0 { Piece::EMPTY } else { self.get_pc(to_bb) };
        let side = usize::from(self.c);

        // update state
        self.rights &= CASTLE_MASK[to].load(Relaxed) & CASTLE_MASK[from].load(Relaxed);
        self.halfm = u8::from(moved > Piece::PAWN && mov.flag != Flag::CAP) * (self.halfm + 1);
        self.enp_sq = 0;
        self.c = !self.c;

        // move piece
        self.toggle(side, moved, from_bb ^ to_bb);
        self.hash ^= ZVALS.pcs[side][moved][from] ^ ZVALS.pcs[side][moved][to];
        self.pst +=      PST[side][moved][to  ];
        self.pst += -1 * PST[side][moved][from];

        // captures
        if captured != Piece::EMPTY {
            let opp = side ^ 1;
            self.toggle(opp, captured, to_bb);
            self.hash ^= ZVALS.pcs[opp][captured][to];
            self.pst += -1 * PST[opp][captured][to];
            self.phase -= PHASE_VALS[captured];
        }

        // more complex moves
        match mov.flag {
            Flag::DBL => self.enp_sq = mov.to ^ 8,
            Flag::KS | Flag::QS => {
                let (idx, sf) = (usize::from(mov.flag == Flag::KS), 56 * side);
                let rfr = sf + ROOK_FILES[side][idx].load(Relaxed) as usize;
                let rto = sf + [3, 5][idx];
                self.toggle(side, Piece::ROOK, (1 << rfr) ^ (1 << rto));
                self.hash ^= ZVALS.pcs[side][Piece::ROOK][rfr] ^ ZVALS.pcs[side][Piece::ROOK][rto];
                self.pst += -1 * PST[side][Piece::ROOK][rfr];
                self.pst +=      PST[side][Piece::ROOK][rto];
            },
            Flag::ENP => {
                let pawn_sq = to ^ 8;
                self.toggle(side ^ 1, Piece::PAWN, 1 << pawn_sq);
                self.hash ^= ZVALS.pcs[side ^ 1][Piece::PAWN][pawn_sq];
                self.pst += -1 * PST[side ^ 1][Piece::PAWN][pawn_sq];
            },
            Flag::PROMO.. => {
                let promo = usize::from((mov.flag & 3) + 3);
                self.bb[Piece::PAWN] ^= to_bb;
                self.bb[promo] ^= to_bb;
                self.hash ^= ZVALS.pcs[side][Piece::PAWN][to] ^ ZVALS.pcs[side][promo][to];
                self.pst += -1 * PST[side][Piece::PAWN][to];
                self.pst +=      PST[side][promo      ][to];
                self.phase += PHASE_VALS[promo];
            }
            _ => {}
        }

        // validating move
        let kidx = (self.bb[Piece::KING] & self.bb[side]).trailing_zeros() as usize;
        self.sq_attacked(kidx, side, self.bb[0] | self.bb[1])
    }

    pub fn mat_draw(&self) -> bool {
        let (ph, b) = (self.phase, self.bb[Piece::BISHOP]);
        ph <= 2 && self.bb[Piece::PAWN] == 0 && ((ph != 2) // no pawns left, phase <= 2
            || (b & self.bb[Side::WHITE] != b && b & self.bb[Side::BLACK] != b // one bishop each
                && (b & 0x55AA55AA55AA55AA == b || b & 0xAA55AA55AA55AA55 == b))) // same colour bishops
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[Piece::KING] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.sq_attacked(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

    pub fn eval(&self) -> i16 {
        let (s, p) = (self.pst, 24.min(i32::from(self.phase)));
        SIDE[usize::from(self.c)] * ((p * s.0 as i32 + (24 - p) * s.1 as i32) / 24) as i16
    }

    pub fn movegen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::default();
        let (side, occ) = (usize::from(self.c), self.bb[0] | self.bb[1]);
        let (boys, opps) = (self.bb[side], self.bb[side ^ 1]);
        let pawns = self.bb[Piece::PAWN] & boys;

        // special quiet moves
        if QUIETS {
            let r = self.rights;
            if r & [Rights::WHITE, Rights::BLACK][side] > 0 && !self.in_check() {
                let kbb = self.bb[Piece::KING] & self.bb[side];
                let ksq = kbb.trailing_zeros() as u8;
                if self.c {
                    if self.castle(Rights::BQS, 0, occ, kbb, 1 << 58, 1 << 59) {moves.push(ksq, 58, Flag::QS, Piece::KING)}
                    if self.castle(Rights::BKS, 1, occ, kbb, 1 << 62, 1 << 61) {moves.push(ksq, 62, Flag::KS, Piece::KING)}
                } else {
                    if self.castle(Rights::WQS, 0, occ, kbb, 1 << 2, 1 << 3) {moves.push(ksq, 2, Flag::QS, Piece::KING)}
                    if self.castle(Rights::WKS, 1, occ, kbb, 1 << 6, 1 << 5) {moves.push(ksq, 6, Flag::KS, Piece::KING)}
                }
            }

            // pawn pushes
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

        // pawn captures
        if self.enp_sq > 0 {
            let mut attackers = Attacks::PAWN[side ^ 1][self.enp_sq as usize] & pawns;
            bitloop!(attackers, from, moves.push(from, self.enp_sq, Flag::ENP, Piece::PAWN));
        }
        let (mut attackers, mut promo) = (pawns & !Rank::PEN[side], pawns & Rank::PEN[side]);
        bitloop!(attackers, from,
            encode::<{ Flag::CAP }>(&mut moves, Attacks::PAWN[side][from as usize] & opps, from, Piece::PAWN)
        );
        bitloop!(promo, from, {
            let mut attacks = Attacks::PAWN[side][from as usize] & opps;
            bitloop!(attacks, to, for flag in Flag::NPC..=Flag::QPC { moves.push(from, to, flag, Piece::PAWN) });
        });

        // non-pawn moves
        for pc in Piece::KNIGHT..=Piece::KING {
            let mut attackers = boys & self.bb[pc];
            bitloop!(attackers, from, {
                let attacks = match pc {
                    Piece::KNIGHT => Attacks::KNIGHT[from as usize],
                    Piece::ROOK   => Attacks::rook  (from as usize, occ),
                    Piece::BISHOP => Attacks::bishop(from as usize, occ),
                    Piece::QUEEN  => Attacks::bishop(from as usize, occ) | Attacks::rook(from as usize, occ),
                    Piece::KING   => Attacks::KING  [from as usize],
                    _ => unreachable!(),
                };
                encode::<{ Flag::CAP }>(&mut moves, attacks & opps, from, pc);
                if QUIETS { encode::<{ Flag::QUIET }>(&mut moves, attacks & !occ, from, pc) }
            });
        }
        moves
    }

    fn path(&self, side: usize, mut path: u64, occ: u64) -> bool {
        bitloop!(path, idx, if self.sq_attacked(idx as usize, side, occ) {return false});
        true
    }

    fn castle(&self, right: u8, ks: usize, occ: u64, kbb: u64, kto: u64, rto: u64) -> bool {
        let side = usize::from(self.c);
        let bit = 1 << (56 * side + usize::from(ROOK_FILES[side][ks].load(Relaxed)));
        self.rights & right > 0
            && (occ ^ bit) & (btwn(kbb, kto) ^ kto) == 0
            && (occ ^ kbb) & (btwn(bit, rto) ^ rto) == 0
            && self.path(side, btwn(kbb, kto), occ)
    }

    pub fn from_fen(fen: &str) -> Self {
        let vec = fen.split_whitespace().collect::<Vec<&str>>();
        let p = vec[0].chars().collect::<Vec<char>>();

        // board
        let (mut pos, mut row, mut col) = (Self::default(), 7, 0);
        for ch in p {
            if ch == '/' { row -= 1; col = 0; }
            else if ('1'..='8').contains(&ch) { col += ch.to_string().parse().unwrap_or(0) }
            else if let Some(idx) = ['P','N','B','R','Q','K','p','n','b','r','q','k'].iter().position(|&el| el == ch) {
                let side = usize::from(idx > 5);
                let (pc, sq) = (idx + 2 - 6 * side, 8 * row + col);
                pos.toggle(side, pc, 1 << sq);
                pos.hash ^= ZVALS.pcs[side][pc][sq as usize];
                pos.pst += PST[side][pc][sq as usize];
                pos.phase += PHASE_VALS[pc];
                col += 1;
            }
        }

        // state
        pos.c = vec[1] == "b";
        pos.enp_sq = if vec[3] == "-" {0} else {sq_to_idx(vec[3])};
        pos.halfm = vec.get(4).unwrap_or(&"0").parse::<u8>().unwrap();

        // general castling stuff (for chess960)
        let mut king = 4;
        CHESS960.store(false, Relaxed);
        ROOK_FILES[0][0].store(0, Relaxed);
        ROOK_FILES[0][1].store(7, Relaxed);
        ROOK_FILES[1][0].store(0, Relaxed);
        ROOK_FILES[1][1].store(7, Relaxed);
        pos.rights = vec[2].chars().fold(0, |cr, ch| cr | match ch as u8 {
            b'Q' => Rights::WQS, b'K' => Rights::WKS, b'q' => Rights::BQS, b'k' => Rights::BKS,
            b'A'..=b'H' => pos.parse_castle(Side::WHITE, &mut king, ch),
            b'a'..=b'h' => pos.parse_castle(Side::BLACK, &mut king, ch),
            _ => 0
        });
        for sq in &CASTLE_MASK { sq.store(15, Relaxed) }
        CASTLE_MASK[usize::from(     ROOK_FILES[0][0].load(Relaxed))].store( 7, Relaxed);
        CASTLE_MASK[usize::from(     ROOK_FILES[0][1].load(Relaxed))].store(11, Relaxed);
        CASTLE_MASK[usize::from(56 + ROOK_FILES[1][0].load(Relaxed))].store(13, Relaxed);
        CASTLE_MASK[usize::from(56 + ROOK_FILES[1][1].load(Relaxed))].store(14, Relaxed);
        CASTLE_MASK[     king].store( 3, Relaxed);
        CASTLE_MASK[56 + king].store(12, Relaxed);

        pos
    }

    fn parse_castle(&self, side: usize, king: &mut usize, ch: char) -> u8 {
        CHESS960.store(true, Relaxed);
        let wkc = (self.bb[side] & self.bb[Piece::KING]).trailing_zeros() as u8 & 7;
        *king = wkc as usize;
        let rook = ch as u8 - [b'A', b'a'][side];
        let i = usize::from(rook > wkc);
        ROOK_FILES[side][i].store(rook, Relaxed);
        [[Rights::WQS, Rights::WKS], [Rights::BQS, Rights::BKS]][side][i]
    }
}

fn encode<const FLAG: u8>(moves: &mut MoveList, mut attacks: u64, from: u8, pc: usize) {
    bitloop!(attacks, to, moves.push(from, to, FLAG, pc));
}

fn shift(side: usize,bb: u64) -> u64 {
    if side == Side::WHITE { bb >> 8 } else { bb << 8 }
}

fn idx_shift<const AMOUNT: u8>(side: usize, idx: u8) -> u8 {
    if side == Side::WHITE { idx + AMOUNT } else { idx - AMOUNT }
}

fn btwn(bit1: u64, bit2: u64) -> u64 {
    let min = bit1.min(bit2);
    (bit1.max(bit2) - min) ^ min
}

fn sq_to_idx(sq: &str) -> u8 {
    let chs: Vec<char> = sq.chars().collect();
    8 * chs[1].to_string().parse::<u8>().unwrap() + chs[0] as u8 - 105
}

impl Move {
    pub fn from_short(m: u16, pos: &Position) -> Self {
        let from = ((m >> 6) & 63) as u8;
        Self { from, to: (m & 63) as u8, flag: (m >> 12) as u8, pc: pos.get_pc(1 << from) as u8 }
    }

    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {["n","b","r","q"][(self.flag & 0b11) as usize]} else {""};
        let to = if CHESS960.load(Relaxed) && [Flag::QS, Flag::KS].contains(&self.flag) {
            let sf = 56 * (self.to / 56);
            sf + ROOK_FILES[usize::from(sf > 0)][usize::from(self.flag == Flag::KS)].load(Relaxed)
        } else { self.to };
        format!("{}{}{} ", idx_to_sq(self.from), idx_to_sq(to), promo)
    }

    pub fn from_uci(pos: &Position, m_str: &str) -> Self {
        let (from, to, c) = (sq_to_idx(&m_str[0..2]), sq_to_idx(&m_str[2..4]), usize::from(pos.c));

        if CHESS960.load(Relaxed) && pos.bb[c] & (1 << to) > 0 {
            let side = 56 * (from / 56);
            let (to2, flag) = if to == ROOK_FILES[c][0].load(Relaxed) + side {
                (2 + side, Flag::QS)
            } else { (6 + side, Flag::KS) };
            return Move { from, to: to2, flag, pc: Piece::KING as u8};
        }

        let mut m = Move { from, to, flag: 0, pc: 0};
        m.flag = match m_str.chars().nth(4).unwrap_or('f') {'n' => 8, 'b' => 9, 'r' => 10, 'q' => 11, _ => 0};
        let possible_moves = pos.movegen::<true>();
        *possible_moves.list.iter().take(possible_moves.len).find(|um|
            m.from == um.from && m.to == um.to && (m_str.len() < 5 || m.flag == um.flag & 0b1011)
            && !(CHESS960.load(Relaxed) && [Flag::QS, Flag::KS].contains(&um.flag))
        ).unwrap()
    }
}
