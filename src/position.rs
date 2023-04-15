use super::{consts::*, eval::*};

#[derive(Clone, Copy, Default)]
pub struct State {
    hash: u64,
    pst: S,
    hfm: u8,
    pub enp: u8,
    pub cr: u8,
}

pub struct MoveCtx(State, Move, u8);

#[derive(Default)]
pub struct Position {
    pub bb: [u64; 8],
    pub c: bool,
    pub state: State,
    pub phase: i16,
    stack: Vec<MoveCtx>,
    nulls: u16,
    zvals: Box<ZobristVals>,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub flag: u8,
    pub mpc: u8,
}

pub struct ZobristVals {
    pcs: [[[u64; 64]; 8]; 2],
    cr: [u64; 4],
    enp: [u64; 8],
    c: u64,
}

#[inline(always)]
pub fn batt(idx: usize, occ: u64) -> u64 {
    let m = MASKS[idx];
    let rb = m.bit.swap_bytes();
    let (mut f1, mut f2) = (occ & m.diag, occ & m.anti);
    let (r1, r2) = (f1.swap_bytes() - rb, f2.swap_bytes() - rb);
    f1 -= m.bit;
    f2 -= m.bit;
    ((f1 ^ r1.swap_bytes()) & m.diag) | ((f2 ^ r2.swap_bytes()) & m.anti)
}

#[inline(always)]
pub fn ratt(idx: usize, occ: u64) -> u64 {
    let m = MASKS[idx];
    let mut f = occ & m.file;
    let i = idx & 7;
    let s = idx - i;
    let r = f.swap_bytes() - m.bit.swap_bytes();
    f -= m.bit;
    ((f ^ r.swap_bytes()) & m.file) | (RANKS[i][((occ >> (s + 1)) & 0x3F) as usize] << s)
}

impl Position {
    pub fn from_fen(fen: &str) -> Self {
        let vec = fen.split_whitespace().collect::<Vec<&str>>();
        let p = vec[0].chars().collect::<Vec<char>>();
        let (mut pos, mut row, mut col) = (Self::default(), 7, 0);
        for ch in p {
            if ch == '/' { row -= 1; col = 0; }
            else if ('1'..='8').contains(&ch) { col += ch.to_string().parse::<i16>().unwrap_or(0) }
            else {
                let idx = ['P','N','B','R','Q','K','p','n','b','r','q','k'].iter().position(|&el| el == ch).unwrap_or(6);
                let side = usize::from(idx > 5);
                let (pc, sq) = (idx + 2 - 6 * side, 8 * row + col);
                pos.toggle(side, pc, 1 << sq);
                pos.state.hash ^= pos.zvals.pcs[side][pc][sq as usize];
                pos.state.pst += SIDE[side] * PST[pc][sq as usize ^ (56 * (side^ 1))];
                pos.phase += PHASE_VALS[pc];
                col += 1;
            }
        }
        pos.c = vec[1] == "b";
        pos.state.cr = vec[2].chars().fold(0, |cr, ch| cr | match ch {'Q' => WQS, 'K' => WKS, 'q' => BQS, 'k' => BKS, _ => 0});
        pos.state.enp = if vec[3] == "-" {0} else {sq_to_idx(vec[3])};
        pos.state.hfm = vec.get(4).unwrap_or(&"0").parse::<u8>().unwrap();
        pos
    }

    pub fn hash(&self) -> u64 {
        let (mut hash, mut r) = (self.state.hash, self.state.cr);
        if self.c {hash ^= self.zvals.c}
        if self.state.enp > 0 {hash ^= self.zvals.enp[self.state.enp as usize & 7]}
        while r > 0 {
            hash ^= self.zvals.cr[r.trailing_zeros() as usize];
            r &= r - 1;
        }
        hash
    }

    #[inline]
    fn toggle(&mut self, c: usize, pc: usize, bit: u64) {
        self.bb[pc] ^= bit;
        self.bb[c] ^= bit;
    }

    #[inline]
    pub fn is_sq_att(&self, idx: usize, side: usize, occ: u64) -> bool {
        let s = self.bb[side ^ 1];
           (NATT[idx] & self.bb[N] & s > 0)
        || (KATT[idx] & self.bb[K] & s > 0)
        || (PATT[side][idx] & self.bb[P] & s > 0)
        || (ratt(idx, occ) & ((self.bb[R] | self.bb[Q]) & s) > 0)
        || (batt(idx, occ) & ((self.bb[B] | self.bb[Q]) & s) > 0)
    }

    #[inline]
    pub fn get_pc(&self, bit: u64) -> usize {
        usize::from((self.bb[N] | self.bb[R] | self.bb[K]) & bit > 0)
        | (2 * usize::from((self.bb[N] | self.bb[P] | self.bb[Q] | self.bb[K]) & bit > 0))
        | (4 * usize::from((self.bb[B] | self.bb[R] | self.bb[Q] | self.bb[K]) & bit > 0))
    }

    pub fn r#do(&mut self, m: Move) -> bool {
        let cpc = if m.flag & CAP == 0 || m.flag == ENP {E} else {self.get_pc(1 << m.to)};
        let side = usize::from(self.c);
        self.stack.push(MoveCtx(self.state, m, cpc as u8));
        self.state.cr &= CR[m.to as usize] & CR[m.from as usize];
        self.state.enp = if m.flag == DBL {if side == WH {m.to - 8} else {m.to + 8}} else {0};
        self.state.hfm = u8::from(m.mpc > P as u8 && m.flag != CAP) * (self.state.hfm + 1);
        self.r#move::<true>(m, side, cpc);
        let kidx = (self.bb[K] & self.bb[side]).trailing_zeros() as usize;
        let invalid = self.is_sq_att(kidx, side, self.bb[0] | self.bb[1]);
        if invalid {self.undo()}
        invalid
    }

    pub fn undo(&mut self) {
        let ctx = self.stack.pop().unwrap();
        self.state = ctx.0;
        self.r#move::<false>(ctx.1, usize::from(!self.c), ctx.2 as usize);
    }

    fn r#move<const DO: bool>(&mut self, m: Move, side: usize, cpc: usize) {
        let (sign, psign, flip) = (SIDE[usize::from(!DO)], SIDE[side], 56 * (side ^ 1));
        let (f, t, mpc) = (1 << m.from, 1 << m.to, usize::from(m.mpc));
        self.c = !self.c;
        self.toggle(side, mpc, f | t);
        if DO {
            self.state.hash ^= self.zvals.pcs[side][mpc][usize::from(m.from)] ^ self.zvals.pcs[side][mpc][usize::from(m.to)];
            self.state.pst += psign * PST[mpc][usize::from(m.to) ^ flip];
            self.state.pst += -psign * PST[mpc][usize::from(m.from) ^ flip];
        }
        if cpc != E {
            self.toggle(side ^ 1, cpc, t);
            if DO {
                self.state.hash ^= self.zvals.pcs[side ^ 1][cpc][usize::from(m.to)];
                self.state.pst += psign * PST[cpc][usize::from(m.to) ^ (56 * side)];
            }
            self.phase -= sign * PHASE_VALS[cpc];
        }
        match m.flag {
            KS | QS => {
                let (bits, idx1, idx2) = CM[usize::from(m.flag == KS)][side];
                self.toggle(side, R, bits);
                if DO {
                    self.state.hash ^= self.zvals.pcs[side][R][idx1] ^ self.zvals.pcs[side][R][idx2];
                    self.state.pst += -psign * PST[R][idx1 ^ flip];
                    self.state.pst += psign * PST[R][idx2 ^ flip];
                }
            },
            ENP => {
                let pwn = usize::from(m.to + [8u8.wrapping_neg(), 8][side]);
                self.toggle(side ^ 1, P, 1 << pwn);
                if DO {
                    self.state.hash ^= self.zvals.pcs[side ^ 1][P][pwn];
                    self.state.pst += psign * PST[P][pwn ^ (56 * side)];
                }
            },
            NPR.. => {
                let ppc = usize::from((m.flag & 3) + 3);
                self.bb[P] ^= t;
                self.bb[ppc] ^= t;
                if DO {
                    self.state.hash ^= self.zvals.pcs[side][P][usize::from(m.to)] ^ self.zvals.pcs[side][ppc][usize::from(m.to)];
                    self.state.pst += -psign * PST[P][usize::from(m.to) ^ flip];
                    self.state.pst += psign * PST[ppc][usize::from(m.to) ^ flip];
                }
                self.phase += sign * PHASE_VALS[ppc];
            }
            _ => {}
        }
    }

    pub fn r#do_null(&mut self) {
        self.nulls += 1;
        self.stack.push(MoveCtx(self.state, Move::default(), 0));
        self.state.enp = 0;
        self.c = !self.c;
    }

    pub fn undo_null(&mut self) {
        self.nulls -= 1;
        let ctx = self.stack.pop().unwrap();
        self.state.enp = ctx.0.enp;
        self.c = !self.c;
    }

    fn rep_draw(&self, ply: i16) -> bool {
        let mut num = 1 + 2 * u8::from(ply == 0);
        let l = self.stack.len();
        if l < 6 || self.nulls > 0 { return false }
        for ctx in self.stack.iter().rev().take(self.state.hfm as usize + 1).skip(1).step_by(2) {
            num -= u8::from(ctx.0.hash == self.state.hash);
            if num == 0 { return true }
        }
        false
    }

    fn mat_draw(&self) -> bool {
        let (ph, b, p, wh, bl) = (self.phase, self.bb[B], self.bb[P], self.bb[WH], self.bb[BL]);
        ph <= 2 && p == 0 && ((ph != 2) || (b & wh != b && b & bl != b && (b & LSQ == b || b & DSQ == b)))
    }

    pub fn is_draw(&self, ply: i16) -> bool {
        self.state.hfm >= 100 || self.rep_draw(ply) || self.mat_draw()
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[K] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.is_sq_att(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

    pub fn lazy_eval(&self) -> i16 {
        let (score, phase) = (self.state.pst, std::cmp::min(self.phase as i32, TPHASE));
        SIDE[usize::from(self.c)] * ((phase * score.0 as i32 + (TPHASE - phase) * score.1 as i32) / TPHASE) as i16
    }
}

fn sq_to_idx(sq: &str) -> u8 {
    let chs: Vec<char> = sq.chars().collect();
    8 * chs[1].to_string().parse::<u8>().unwrap() + chs[0] as u8 - 105
}

impl Move {
    pub fn from_short(m: u16, pos: &Position) -> Self {
        let from = ((m >> 6) & 63) as u8;
        Self { from, to: (m & 63) as u8, flag: ((m >> 12) & 63) as u8, mpc: pos.get_pc(1 << from) as u8 }
    }

    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {["n","b","r","q"][(self.flag & 0b11) as usize]} else {""};
        format!("{}{}{} ", idx_to_sq(self.from), idx_to_sq(self.to), promo)
    }

    pub fn from_uci(pos: &Position, m_str: &str) -> Self {
        let mut m = Move { from: sq_to_idx(&m_str[0..2]), to: sq_to_idx(&m_str[2..4]), flag: 0, mpc: 0};
        m.flag = match m_str.chars().nth(4).unwrap_or('f') {'n' => 8, 'b' => 9, 'r' => 10, 'q' => 11, _ => 0};
        let possible_moves = pos.gen::<ALL>();
        *possible_moves.list.iter().take(possible_moves.len).find(|um|
            m.from == um.from && m.to == um.to && (m_str.len() < 5 || m.flag == um.flag & 0b1011)
        ).unwrap()
    }
}

fn random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

impl Default for ZobristVals {
    fn default() -> Self {
        let mut seed = 180_620_142;
        let mut vals = Self { pcs: [[[0; 64]; 8]; 2], cr: [0; 4], enp: [0; 8], c: random(&mut seed) };
        vals.pcs.iter_mut().for_each(|s| s.iter_mut().skip(2).for_each(|p| p.iter_mut().for_each(|sq| *sq = random(&mut seed))));
        vals.cr.iter_mut().for_each(|r| *r = random(&mut seed));
        vals.enp.iter_mut().for_each(|f| *f = random(&mut seed));
        vals
    }
}
