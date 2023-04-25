use super::consts::*;

#[derive(Clone, Copy, Default)]
pub struct Position {
    pub bb: [u64; 8],
    pub c: bool,
    pub hfm: u8,
    pub enp: u8,
    pub cr: u8,
    pub phase: i16,
    pub nulls: u16,
    pub hash: u64,
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
        pos.c = vec[1] == "b";
        pos.cr = vec[2].chars().fold(0, |cr, ch| cr | match ch {'Q' => WQS, 'K' => WKS, 'q' => BQS, 'k' => BKS, _ => 0});
        pos.enp = if vec[3] == "-" {0} else {sq_to_idx(vec[3])};
        pos.hfm = vec.get(4).unwrap_or(&"0").parse::<u8>().unwrap();
        pos
    }

    pub fn hash(&self) -> u64 {
        let (mut hash, mut r) = (self.hash, self.cr);
        if self.c {hash ^= ZVALS.c}
        if self.enp > 0 {hash ^= ZVALS.enp[self.enp as usize & 7]}
        while r > 0 {
            hash ^= ZVALS.cr[r.trailing_zeros() as usize];
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

    pub fn make(&mut self, m: Move) -> bool {
        let (f, t, mpc) = (1 << m.from, 1 << m.to, usize::from(m.mpc));
        let (to, from) = (usize::from(m.to), usize::from(m.from));
        let cpc = if m.flag & CAP == 0 || m.flag == ENP {E} else {self.get_pc(t)};
        let side = usize::from(self.c);

        // update state
        self.cr &= CR[to] & CR[from];
        self.enp = if m.flag == DBL {if side == WH {m.to - 8} else {m.to + 8}} else {0};
        self.hfm = u8::from(mpc > P && m.flag != CAP) * (self.hfm + 1);
        self.c = !self.c;

        // move piece
        self.toggle(side, mpc, f | t);
        self.hash ^= ZVALS.pcs[side][mpc][from] ^ ZVALS.pcs[side][mpc][to];
        self.pst += PST[side][mpc][to];
        self.pst += -1 * PST[side][mpc][from];

        // captures
        if cpc != E {
            self.toggle(side ^ 1, cpc, t);
            self.hash ^= ZVALS.pcs[side ^ 1][cpc][to];
            self.pst += -1 * PST[side ^ 1][cpc][to];
            self.phase -= PHASE_VALS[cpc];
        }

        // more complex moves
        match m.flag {
            KS | QS => {
                let (bits, rfr, rto) = CM[usize::from(m.flag == KS)][side];
                self.toggle(side, R, bits);
                self.hash ^= ZVALS.pcs[side][R][rfr] ^ ZVALS.pcs[side][R][rto];
                self.pst += -1 * PST[side][R][rfr];
                self.pst += PST[side][R][rto];
            },
            ENP => {
                let pwn = to + [8usize.wrapping_neg(), 8][side];
                self.toggle(side ^ 1, P, 1 << pwn);
                self.hash ^= ZVALS.pcs[side ^ 1][P][pwn];
                self.pst += -1 * PST[side ^ 1][P][pwn];
            },
            NPR.. => {
                let ppc = usize::from((m.flag & 3) + 3);
                self.bb[P] ^= t;
                self.bb[ppc] ^= t;
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

    pub fn make_null(&mut self) {
        self.nulls += 1;
        self.enp = 0;
        self.c = !self.c;
    }

    pub fn mat_draw(&self) -> bool {
        let (ph, b, p, wh, bl) = (self.phase, self.bb[B], self.bb[P], self.bb[WH], self.bb[BL]);
        ph <= 2 && p == 0 && ((ph != 2) || (b & wh != b && b & bl != b && (b & LSQ == b || b & DSQ == b)))
    }

    pub fn in_check(&self) -> bool {
        let kidx = (self.bb[K] & self.bb[usize::from(self.c)]).trailing_zeros() as usize;
        self.is_sq_att(kidx, usize::from(self.c), self.bb[0] | self.bb[1])
    }

    pub fn lazy_eval(&self) -> i16 {
        let (score, phase) = (self.pst, TPHASE.min(self.phase as i32));
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
        Self { from, to: (m & 63) as u8, flag: (m >> 12) as u8, mpc: pos.get_pc(1 << from) as u8 }
    }

    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {["n","b","r","q"][(self.flag & 0b11) as usize]} else {""};
        format!("{}{}{}", idx_to_sq(self.from), idx_to_sq(self.to), promo)
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
