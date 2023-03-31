use super::consts::*;

#[derive(Clone, Copy, Default)]
pub struct State {
    hash: u64,
    pub pst: S,
    pub enp: u8,
    pub cr: u8,
    pub hfm: u8,
}

#[derive(Clone, Copy)]
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
    pub pieces: [[[u64; 64]; 8]; 2],
    pub cr: [u64; 4],
    pub enp: [u64; 8],
    pub c: u64,
}

#[inline(always)]
pub fn batt(idx: usize, occ: u64) -> u64 {
    let m = BMASKS[idx];
    let mut f = occ & m.right;
    let mut r = f.swap_bytes();
    f -= m.bit;
    r -= m.file;
    f ^= r.swap_bytes();
    f &= m.right;
    let mut f2 = occ & m.left;
    r = f2.swap_bytes();
    f2 -= m.bit;
    r -= m.file;
    f2 ^= r.swap_bytes();
    f2 &= m.left;
    f | f2
}

#[inline(always)]
pub fn ratt(idx: usize, occ: u64) -> u64 {
    let m = RMASKS[idx];
    let mut f = occ & m.file;
    let mut r = f.swap_bytes();
    f -= m.bit;
    r -= m.bit.swap_bytes();
    f ^= r.swap_bytes();
    f &= m.file;
    let mut e = m.right & occ;
    r = e & e.wrapping_neg();
    e = (r ^ (r - m.bit)) & m.right;
    let w = m.left ^ WEST[(((m.left & occ)| 1).leading_zeros() ^ 63) as usize];
    f | e | w
}

impl Position {
    pub fn from_fen(fen: &str) -> Self {
        let mut pos = Self::default();
        let vec: Vec<&str> = fen.split_whitespace().collect();
        let p: Vec<char> = vec[0].chars().collect();
        let (mut row, mut col) = (7, 0);
        for ch in p {
            if ch == '/' { row -= 1; col = 0; }
            else if ('1'..='8').contains(&ch) { col += ch.to_string().parse::<i16>().unwrap_or(0) }
            else {
                let idx = ['P','N','B','R','Q','K','p','n','b','r','q','k'].iter().position(|&element| element == ch).unwrap_or(6);
                let side = usize::from(idx > 5);
                let pc = idx + 2 - 6 * side;
                let sq = 8 * row + col;
                pos.toggle(side, pc, 1 << sq);
                pos.state.hash ^= pos.zvals.pieces[side][pc][sq as usize];
                pos.state.pst += SIDE[side] * PST[pc][sq as usize ^ (56 * (side^ 1))];
                pos.phase += PHASE_VALS[pc];
                col += 1;
            }
        }
        pos.c = vec[1] == "b";
        for ch in vec[2].chars() {pos.state.cr |= match ch {'Q' => WQS, 'K' => WKS, 'q' => BQS, 'k' => BKS, _ => 0}}
        pos.state.enp = if vec[3] == "-" {0} else {
            let chs: Vec<char> = vec[3].chars().collect();
            8 * chs[1].to_string().parse::<u8>().unwrap_or(0) + chs[0] as u8 - 105
        };
        pos.state.hfm = vec[4].parse::<u8>().unwrap();
        pos
    }

    pub fn hash(&self) -> u64 {
        let mut hash = self.state.hash;
        if self.c {hash ^= self.zvals.c}
        if self.state.enp > 0 {hash ^= self.zvals.enp[self.state.enp as usize & 7]}
        let mut r = self.state.cr;
        while r > 0 {
            hash ^= self.zvals.cr[r.trailing_zeros() as usize];
            r &= r - 1;
        }
        hash
    }

    #[inline(always)]
    pub fn toggle(&mut self, c: usize, pc: usize, bit: u64) {
        self.bb[pc] ^= bit;
        self.bb[c] ^= bit;
    }

    #[inline(always)]
    pub fn is_sq_att(&self, idx: usize, side: usize, occ: u64) -> bool {
        let s = self.bb[side ^ 1];
        (NATT[idx] & self.bb[N] & s > 0)
        || (KATT[idx] & self.bb[K] & s > 0)
        || (PATT[side][idx] & self.bb[P] & s > 0)
        || (ratt(idx, occ) & ((self.bb[R] | self.bb[Q]) & s) > 0)
        || (batt(idx, occ) & ((self.bb[B] | self.bb[Q]) & s) > 0)
    }

    #[inline(always)]
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

    #[inline(always)]
    fn r#move<const DO: bool>(&mut self, m: Move, side: usize, cpc: usize) {
        let sign = SIDE[usize::from(!DO)];
        let psign = SIDE[side];
        let (noflip, flip) = (56 * side, 56 * (side ^ 1));
        let f = 1 << m.from;
        let t = 1 << m.to;
        let mpc = usize::from(m.mpc);
        self.c = !self.c;
        self.toggle(side, mpc, f | t);
        if DO {
            self.state.hash ^= self.zvals.pieces[side][mpc][usize::from(m.from)] ^ self.zvals.pieces[side][mpc][usize::from(m.to)];
            self.state.pst += psign * PST[mpc][usize::from(m.to) ^ flip];
            self.state.pst += -psign * PST[mpc][usize::from(m.from) ^ flip];
        }
        if cpc != E {
            self.toggle(side ^ 1, cpc, t);
            if DO {
                self.state.hash ^= self.zvals.pieces[side ^ 1][cpc][usize::from(m.to)];
                self.state.pst += psign * PST[cpc][usize::from(m.to) ^ noflip];
            }
            self.phase -= sign * PHASE_VALS[cpc];
        }
        match m.flag {
            KS | QS => {
                let (bits, idx1, idx2) = CM[usize::from(m.flag == KS)][side];
                self.toggle(side, R, bits);
                if DO {
                    self.state.hash ^= self.zvals.pieces[side][R][idx1] ^ self.zvals.pieces[side][R][idx2];
                    self.state.pst += -psign * PST[R][idx1 ^ flip];
                    self.state.pst += psign * PST[R][idx2 ^ flip];
                }
            },
            ENP => {
                let pwn = usize::from(m.to + [8u8.wrapping_neg(), 8][side]);
                self.toggle(side ^ 1, P, 1 << pwn);
                if DO {
                    self.state.hash ^= self.zvals.pieces[side ^ 1][P][pwn];
                    self.state.pst += psign * PST[P][pwn ^ noflip];
                }
            },
            NPR.. => {
                let ppc = usize::from((m.flag & 3) + 3);
                self.bb[P] ^= t;
                self.bb[ppc] ^= t;
                if DO {
                    self.state.hash ^= self.zvals.pieces[side][P][usize::from(m.to)] ^ self.zvals.pieces[side][ppc][usize::from(m.to)];
                    self.state.pst += -psign * PST[P][usize::from(m.to) ^ flip];
                    self.state.pst += psign * PST[ppc][usize::from(m.to) ^ flip];
                }
                self.phase += sign * PHASE_VALS[ppc];
            }
            _ => {}
        }
    }

    pub fn do_null(&mut self) -> u8 {
        let enp = self.state.enp;
        self.nulls += 1;
        self.stack.push(MoveCtx(self.state, Move::default(), 0));
        self.state.enp = 0;
        self.c = !self.c;
        enp
    }

    pub fn undo_null(&mut self, enp: u8) {
        self.nulls -= 1;
        self.stack.pop();
        self.state.enp = enp;
        self.c = !self.c;
    }

    pub fn repetition_draw(&self, num: u8) -> bool {
        let l = self.stack.len();
        if l < 6 || self.nulls > 0 { return false }
        let mut reps: u8 = 1;
        for i in (l.saturating_sub(self.state.hfm as usize)..(l - 1)).rev().step_by(2) {
            reps += u8::from(self.stack[i].0.hash == self.state.hash);
            if reps >= num { return true }
        }
        false
    }

    pub fn material_draw(&self) -> bool {
        let (ph, b, p, wh, bl) = (self.phase, self.bb[B], self.bb[P], self.bb[0], self.bb[1]);
        ph <= 2 && p == 0 && ((ph != 2) || (b & wh != b && b & bl != b && (b & LSQ == b || b & DSQ == b)))
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[K] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.is_sq_att(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }
}

macro_rules! idx_to_sq {($idx:expr) => {format!("{}{}", char::from_u32(($idx & 7) as u32 + 97).unwrap(), ($idx >> 3) + 1)}}
fn sq_to_idx(sq: &str) -> u8 {
    let chs: Vec<char> = sq.chars().collect();
    8 * chs[1].to_string().parse::<u8>().unwrap() + chs[0] as u8 - 105
}

impl Move {
    pub fn from_short(m: u16, pos: &Position) -> Self {
        let from = ((m >> 6) & 63) as u8;
        Self { from, to: (m & 63) as u8, flag: ((m >> 12) & 63) as u8, mpc: pos.get_pc(1 << from) as u8 }
    }

    pub fn to_uci(m: Self) -> String {
        let promo = if m.flag & 0b1000 > 0 {["n","b","r","q"][(m.flag & 0b11) as usize]} else {""};
        format!("{}{}{} ", idx_to_sq!(m.from), idx_to_sq!(m.to), promo)
    }

    pub fn from_uci(pos: &Position, m_str: &str) -> Self {
        let mut m = Move { from: sq_to_idx(&m_str[0..2]), to: sq_to_idx(&m_str[2..4]), flag: 0, mpc: 0};
        m.flag |= match m_str.chars().nth(4).unwrap_or('f') {'n' => 8, 'b' => 9, 'r' => 10, 'q' => 11, _ => 0};
        let possible_moves = pos.gen::<ALL>();
        for um in &possible_moves.list[0..possible_moves.len] {
            if m.from == um.from && m.to == um.to && (m_str.len() < 5 || m.flag == um.flag & 0b1011) {
                return *um
            }
        }
        panic!("")
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
        let mut vals = Self { pieces: [[[0; 64]; 8]; 2], cr: [0; 4], enp: [0; 8], c: random(&mut seed) };
        for idx in 0..2 {
            for piece in 2..8 {
                for square in 0..64 {
                    vals.pieces[idx][piece][square] = random(&mut seed);
                }
            }
        }
        for idx in 0..4 { vals.cr[idx] = random(&mut seed) }
        for idx in 0..8 { vals.enp[idx] = random(&mut seed) }
        vals
    }
}