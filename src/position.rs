use super::{lsb, consts::*};

#[macro_export]
macro_rules! flag {($m:expr) => {$m & ALL_FLAGS}}
#[macro_export]
macro_rules! from {($m:expr) => {(($m >> 6) & 63) as usize}}
#[macro_export]
macro_rules! to {($m:expr) => {($m & 63) as usize}}
macro_rules! bit {($x:expr) => {1 << $x}}
macro_rules! pop {($x:expr) => {$x &= $x - 1}}

pub struct Pos {
    pub pieces: [u64; 6],
    pub sides: [u64; 2],
    pub squares: [u8; 64],
    pub c: bool,
    pub state: State,
    pub phase: i16,
    pub nulls: u8,
    pub material: [i16; 6],
    pub stack: Vec<MoveContext>,
}

#[derive(Clone, Copy, Default)]
pub struct State {
    pub hash: u64,
    pub enp: u16,
    pub hfm: u8,
    pub cr: u8,
}

#[derive(Clone, Copy)]
pub struct MoveContext {
    state: State,
    m: u16,
    moved_pc: u8,
    captured_pc: u8,
}

#[inline(always)]
pub fn rook_attacks(idx: usize, occ: u64) -> u64 {
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

#[inline(always)]
pub fn bishop_attacks(idx: usize, occ: u64) -> u64 {
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

impl Pos {
    #[inline(always)]
    pub fn is_sq_att(&self, idx: usize, side: usize, occ: u64) -> bool {
        let s = self.sides[side ^ 1];
        let q = self.pieces[QUEEN] & s;
        (KNIGHT_ATTACKS[idx] & self.pieces[KNIGHT] & s > 0)
        || (KING_ATTACKS[idx] & self.pieces[KING] & s > 0)
        || (PAWN_ATTACKS[side][idx] & self.pieces[PAWN] & s > 0)
        || (rook_attacks(idx, occ) & (self.pieces[ROOK] & s | q) > 0)
        || (bishop_attacks(idx, occ) & (self.pieces[BISHOP] & s | q) > 0)
    }

    pub fn in_check(&self) -> bool {
        let king_idx = lsb!(self.pieces[KING] & self.sides[usize::from(self.c)]) as usize;
        self.is_sq_att(king_idx, usize::from(self.c), self.sides[0] | self.sides[1])
    }

    #[inline(always)]
    fn toggle(&mut self, side: usize, pc: usize, bit: u64) {
        self.pieces[pc] ^= bit;
        self.sides[side] ^= bit;
    }

    pub fn r#do(&mut self, m: u16) -> bool {
        let side = usize::from(self.c);
        self.do_unchecked(m);
        let king_idx = lsb!(self.pieces[KING] & self.sides[side]) as usize;
        let invalid = self.is_sq_att(king_idx, side, self.sides[0] | self.sides[1]);
        if invalid { self.undo() }
        invalid
    }

    pub fn do_unchecked(&mut self, m: u16) {
        let from = from!(m);
        let to = to!(m);
        let f = bit!(from);
        let t = bit!(to);
        let moved_pc = self.squares[from];
        let mpc = moved_pc as usize;
        let captured_pc = self.squares[to];
        let flag = flag!(m);
        let rights = self.state.cr;
        let side = usize::from(self.c);

        self.stack.push(MoveContext { state: self.state, m, moved_pc, captured_pc});
        self.toggle(side, mpc, f ^ t);
        self.state.hash ^= ZVALS.pieces[side][mpc][from] ^ ZVALS.pieces[side][mpc][to];
        self.squares[from] = EMPTY as u8;
        self.squares[to] = moved_pc;
        if self.state.enp > 0 {self.state.hash ^= ZVALS.en_passant[(self.state.enp & 7) as usize]}
        self.state.enp = 0;
        self.state.hash ^= ZVALS.side;
        if captured_pc != EMPTY as u8 && flag != KS && flag != QS {
            let cpc = captured_pc as usize;
            self.toggle(side ^ 1, cpc, t);
            self.state.hash ^= ZVALS.pieces[side ^ 1][cpc][to];
            self.phase -= PHASE_VALS[cpc];
            self.material[cpc] += SIDE[side];
        }
        self.state.cr &= CM[from] & CM[to];
        match flag {
            ENP => {
                let pwn = if side == BLACK {to + 8} else {to - 8};
                let p = bit!(pwn);
                self.toggle(side ^ 1, PAWN, p);
                self.state.hash ^= ZVALS.pieces[side ^ 1][PAWN][pwn];
                self.squares[pwn] = EMPTY as u8;
                self.material[PAWN] += SIDE[side];
            }
            DBL => {
                self.state.enp = if side == WHITE {to - 8} else {to + 8} as u16;
                self.state.hash ^= ZVALS.en_passant[to & 7];
            }
            KS | QS => {
                let (bits, idx1, idx2) = CMOV[usize::from(flag == KS)][side];
                self.toggle(side, ROOK, bits);
                self.squares[idx1] = EMPTY as u8;
                self.squares[idx2] = ROOK as u8;
                self.state.hash ^= ZVALS.pieces[side][ROOK][idx1] ^ ZVALS.pieces[side][ROOK][idx2];
            }
            PR.. => {
                let ppc = (((flag >> 12) & 3) + 1) as usize;
                self.pieces[mpc] ^= t;
                self.pieces[ppc] ^= t;
                self.squares[to] = ppc as u8;
                self.phase += PHASE_VALS[ppc];
                self.state.hash ^= ZVALS.pieces[side][mpc][to] ^ ZVALS.pieces[side][ppc][to];
                self.material[PAWN] -= SIDE[side];
                self.material[ppc] += SIDE[side];
            }
            _ => {}
        }
        self.state.hfm = u8::from(moved_pc > PAWN as u8 && flag != CAP) * (self.state.hfm + 1);
        self.c = !self.c;

        let mut changed_castle = rights & !self.state.cr;
        while changed_castle > 0 {
            self.state.hash ^= ZVALS.castle[lsb!(changed_castle) as usize];
            pop!(changed_castle);
        }
    }

    pub fn undo(&mut self) {
        let state = self.stack.pop().unwrap();
        let from = from!(state.m);
        let to = to!(state.m);
        let f = bit!(from);
        let t = bit!(to);
        let flag = flag!(state.m);
        self.c = !self.c;
        let side = usize::from(self.c);

        self.state = state.state;
        self.toggle(side, state.moved_pc as usize, f ^ t);
        self.squares[from] = state.moved_pc;
        self.squares[to] = state.captured_pc;
        if state.captured_pc != EMPTY as u8 && flag != KS && flag != QS {
            let cpc = state.captured_pc as usize;
            self.toggle(side ^ 1, cpc, t);
            self.phase += PHASE_VALS[cpc];
            self.material[cpc] -= SIDE[side];
        }
        match flag {
            ENP => {
                let pwn = if side == BLACK {to + 8} else {to - 8};
                self.toggle(side ^ 1, PAWN, bit!(pwn));
                self.squares[pwn] = PAWN as u8;
                self.material[PAWN] -= SIDE[side];
            }
            KS | QS => {
                let (bits, idx1, idx2) = CMOV[usize::from(flag == KS)][side];
                self.toggle(side, ROOK, bits);
                self.squares[idx1] = ROOK as u8;
                self.squares[idx2] = EMPTY as u8;
            }
            PR.. => {
                let ppc = (((flag >> 12) & 3) + 1) as usize;
                self.pieces[state.moved_pc as usize] ^= t;
                self.pieces[ppc] ^= t;
                self.phase -= PHASE_VALS[ppc];
                self.material[ppc] -= SIDE[side];
                self.material[PAWN] += SIDE[side];
            }
            _ => {}
        }
    }

    pub fn do_null(&mut self, ply: &mut i16) -> u16 {
        self.nulls += 1;
        *ply += 1;
        let enp = self.state.enp;
        self.state.hash ^= u64::from(enp > 0) * ZVALS.en_passant[(enp & 7) as usize];
        self.state.enp = 0;
        self.c = !self.c;
        self.state.hash ^= ZVALS.side;
        enp
    }

    pub fn undo_null(&mut self, enp: u16, hash: u64, ply: &mut i16) {
        self.nulls -= 1;
        *ply -= 1;
        self.state.hash = hash;
        self.state.enp = enp;
        self.c = !self.c;
    }

    fn repetition_draw(&self, num: u8) -> bool {
        let l = self.stack.len();
        if l < 6 || self.nulls > 0 { return false }
        let to = l - 1;
        let from = l.saturating_sub(self.state.hfm as usize);
        let mut reps: u8 = 1;
        for i in (from..to).rev().step_by(2) {
            if self.stack[i].state.hash == self.state.hash {
                reps += 1;
                if reps >= num { return true }
            }
        }
        false
    }

    pub fn material_draw(&self) -> bool {
        if self.phase <= 2 && self.pieces[PAWN] == 0 {
            if self.phase == 2 {
                let b = self.pieces[BISHOP];
                return b & self.sides[0] != b && b & self.sides[1] != b && (b & LSQ == b || b & DSQ == b)
            }
            return true
        }
        false
    }

    pub fn is_draw(&self, ply: i16) -> bool {
        self.state.hfm >= 100 || self.repetition_draw(2 + u8::from(ply == 0)) || self.material_draw()
    }
}
