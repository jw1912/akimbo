use super::consts::*;
use std::sync::atomic::{AtomicU8, AtomicBool, Ordering::Relaxed};

#[allow(clippy::declare_interior_mutable_const)]
const ATOMIC_INIT: AtomicU8 = AtomicU8::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const ATOMIC_INIT_2: [AtomicU8; 2] = [ATOMIC_INIT; 2];
static CHESS960: AtomicBool = AtomicBool::new(false);
static CASTLE_MASK: [AtomicU8; 64] = [ATOMIC_INIT; 64];
pub static ROOK_FILES: [[AtomicU8; 2]; 2] = [ATOMIC_INIT_2; 2];

#[derive(Clone, Copy, Default)]
pub struct Position {
    pub bb: [u64; 8],
    pub c: bool,
    pub halfm: u8,
    pub enp_sq: u8,
    pub rights: u8,
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
    pub mpc: u8,
}

#[inline(always)]
pub fn batt(idx: usize, occ: u64) -> u64 {
    let m = MASKS[idx];
    let rb = m.bit.swap_bytes();
    let (mut f1, mut f2) = (occ & m.diag, occ & m.anti);
    let (r1, r2) = (f1.swap_bytes().wrapping_sub(rb), f2.swap_bytes().wrapping_sub(rb));
    f1 = f1.wrapping_sub(m.bit);
    f2 = f2.wrapping_sub(m.bit);
    ((f1 ^ r1.swap_bytes()) & m.diag) | ((f2 ^ r2.swap_bytes()) & m.anti)
}

#[inline(always)]
pub fn ratt(idx: usize, occ: u64) -> u64 {
    let m = MASKS[idx];
    let mut f = occ & m.file;
    let i = idx & 7;
    let s = idx - i;
    let r = f.swap_bytes().wrapping_sub(m.bit.swap_bytes());
    f = f.wrapping_sub(m.bit);
    ((f ^ r.swap_bytes()) & m.file) | (RANKS[i][((occ >> (s + 1)) & 0x3F) as usize] << s)
}

impl Position {
    pub fn from_fen(fen: &str) -> Self {
        let vec = fen.split_whitespace().collect::<Vec<&str>>();
        let p = vec[0].chars().collect::<Vec<char>>();

        // board
        let (mut pos, mut row, mut col) = (Self::default(), 7, 0);
        for ch in p {
            if ch == '/' { row -= 1; col = 0; }
            else if ('1'..='8').contains(&ch) { col += ch.to_string().parse().unwrap_or(0) }
            else if let Some(idx) = CHARS.iter().position(|&el| el == ch) {
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
            b'Q' => WQS, b'K' => WKS, b'q' => BQS, b'k' => BKS,
            b'A'..=b'H' => pos.handle_castle(WH, &mut king, ch),
            b'a'..=b'h' => pos.handle_castle(BL, &mut king, ch),
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

    fn handle_castle(&self, side: usize, king: &mut usize, ch: char) -> u8 {
        CHESS960.store(true, Relaxed);
        let wkc = (self.bb[side] & self.bb[K]).trailing_zeros() as u8 & 7;
        *king = wkc as usize;
        let rook = ch as u8 - [b'A', b'a'][side];
        let i = usize::from(rook > wkc);
        ROOK_FILES[side][i].store(rook, Relaxed);
        [[WQS, WKS], [BQS, BKS]][side][i]
    }

    pub fn hash(&self) -> u64 {
        let mut hash = self.hash;
        if self.enp_sq > 0 { hash ^= ZVALS.enp[self.enp_sq as usize & 7] }
        hash ^ ZVALS.cr[usize::from(self.rights)] ^ ZVALS.c[usize::from(self.c)]
    }

    #[inline]
    fn toggle(&mut self, c: usize, pc: usize, bit: u64) {
        self.bb[pc] ^= bit;
        self.bb[c] ^= bit;
    }

    #[inline]
    pub fn is_sq_att(&self, idx: usize, side: usize, occ: u64) -> bool {
        ( (NATT[idx] & self.bb[N])
        | (KATT[idx] & self.bb[K])
        | (PATT[side][idx] & self.bb[P])
        | (ratt(idx, occ) & (self.bb[R] | self.bb[Q]))
        | (batt(idx, occ) & (self.bb[B] | self.bb[Q]))
        ) & self.bb[side ^ 1] > 0
    }

    #[inline]
    pub fn get_pc(&self, bit: u64) -> usize {
        usize::from((self.bb[N] | self.bb[R] | self.bb[K]) & bit > 0)
        | (2 * usize::from((self.bb[N] | self.bb[P] | self.bb[Q] | self.bb[K]) & bit > 0))
        | (4 * usize::from((self.bb[B] | self.bb[R] | self.bb[Q] | self.bb[K]) & bit > 0))
    }

    pub fn make(&mut self, mov: Move) -> bool {
        let (from_bb, to_bb, moved) = (1 << mov.from, 1 << mov.to, usize::from(mov.mpc));
        let (to, from) = (usize::from(mov.to), usize::from(mov.from));
        let captured = if mov.flag & CAP == 0 { E } else { self.get_pc(to_bb) };
        let side = usize::from(self.c);

        // update state
        self.rights &= CASTLE_MASK[to].load(Relaxed) & CASTLE_MASK[from].load(Relaxed);
        self.enp_sq = 0;
        self.halfm = u8::from(moved > P && mov.flag != CAP) * (self.halfm + 1);
        self.c = !self.c;

        // move piece
        self.toggle(side, moved, from_bb ^ to_bb);
        self.hash ^= ZVALS.pcs[side][moved][from] ^ ZVALS.pcs[side][moved][to];
        self.pst += PST[side][moved][to];
        self.pst += -1 * PST[side][moved][from];

        // captures
        if captured != E {
            self.toggle(side ^ 1, captured, to_bb);
            self.hash ^= ZVALS.pcs[side ^ 1][captured][to];
            self.pst += -1 * PST[side ^ 1][captured][to];
            self.phase -= PHASE_VALS[captured];
        }

        // more complex moves
        match mov.flag {
            DBL => self.enp_sq = if side == WH {mov.to - 8} else {mov.to + 8},
            KS | QS => {
                let (idx, sf) = (usize::from(mov.flag == KS), 56 * side);
                let rfr = sf + ROOK_FILES[side][idx].load(Relaxed) as usize;
                let rto = sf + RD[idx];
                self.toggle(side, R, (1 << rfr) ^ (1 << rto));
                self.hash ^= ZVALS.pcs[side][R][rfr] ^ ZVALS.pcs[side][R][rto];
                self.pst += -1 * PST[side][R][rfr];
                self.pst += PST[side][R][rto];
            },
            ENP => {
                let pwn = to.wrapping_add([8usize.wrapping_neg(), 8][side]);
                self.toggle(side ^ 1, P, 1 << pwn);
                self.hash ^= ZVALS.pcs[side ^ 1][P][pwn];
                self.pst += -1 * PST[side ^ 1][P][pwn];
            },
            NPR.. => {
                let ppc = usize::from((mov.flag & 3) + 3);
                self.bb[P] ^= to_bb;
                self.bb[ppc] ^= to_bb;
                self.hash ^= ZVALS.pcs[side][P][to] ^ ZVALS.pcs[side][ppc][to];
                self.pst += -1 * PST[side][P][to];
                self.pst += PST[side][ppc][to];
                self.phase += PHASE_VALS[ppc];
            }
            _ => {}
        }

        // validating move
        let kidx = (self.bb[K] & self.bb[side]).trailing_zeros() as usize;
        self.is_sq_att(kidx, side, self.bb[0] | self.bb[1])
    }

    pub fn mat_draw(&self) -> bool {
        let (ph, b, p, wh, bl) = (self.phase, self.bb[B], self.bb[P], self.bb[WH], self.bb[BL]);
        ph <= 2 && p == 0 && ((ph != 2) || (b & wh != b && b & bl != b && (b & LSQ == b || b & DSQ == b)))
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[K] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.is_sq_att(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

    pub fn eval(&self) -> i16 {
        let (s, p) = (self.pst, TPHASE.min(self.phase as i32));
        SIDE[usize::from(self.c)] * ((p * s.0 as i32 + (TPHASE - p) * s.1 as i32) / TPHASE) as i16
    }
}

fn sq_to_idx(sq: &str) -> u8 {
    let chs: Vec<char> = sq.chars().collect();
    8 * chs[1].to_string().parse::<u8>().unwrap() + chs[0] as u8 - 105
}

impl Move {
    pub fn from_short(m: u16, pos: &Position) -> Self {
        let from = ((m >> 6) & 63) as u8;
        Self { from, to: (m & 63) as u8, flag: (m >> 12) as u8, mpc: pos.get_pc(1 << from) as u8 }
    }

    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {["n","b","r","q"][(self.flag & 0b11) as usize]} else {""};
        let to = if CHESS960.load(Relaxed) && [QS, KS].contains(&self.flag) {
            let sf = 56 * (self.to / 56);
            sf + ROOK_FILES[usize::from(sf > 0)][usize::from(self.flag == KS)].load(Relaxed)
        } else { self.to };
        format!("{}{}{} ", idx_to_sq(self.from), idx_to_sq(to), promo)
    }

    pub fn from_uci(pos: &Position, m_str: &str) -> Self {
        let (from, to, c) = (sq_to_idx(&m_str[0..2]), sq_to_idx(&m_str[2..4]), usize::from(pos.c));

        if CHESS960.load(Relaxed) && pos.bb[c] & (1 << to) > 0 {
            let side = 56 * (from / 56);
            let (to2, flag) = if to == ROOK_FILES[c][0].load(Relaxed) + side { (2 + side, QS) } else { (6 + side, KS) };
            return Move { from, to: to2, flag, mpc: K as u8};
        }

        let mut m = Move { from, to, flag: 0, mpc: 0};
        m.flag = match m_str.chars().nth(4).unwrap_or('f') {'n' => 8, 'b' => 9, 'r' => 10, 'q' => 11, _ => 0};
        let possible_moves = pos.gen::<ALL>();
        *possible_moves.list.iter().take(possible_moves.len).find(|um|
            m.from == um.from && m.to == um.to && (m_str.len() < 5 || m.flag == um.flag & 0b1011)
            && !(CHESS960.load(Relaxed) && [QS, KS].contains(&um.flag))
        ).unwrap()
    }
}
