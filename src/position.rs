use super::{lsb, consts::*, movegen::{bishop_attacks, rook_attacks}, zobrist::ZVALS};

macro_rules! from {($m:expr) => {(($m >> 6) & 63) as usize}}
macro_rules! to {($m:expr) => {($m & 63) as usize}}
macro_rules! bit {($x:expr) => {1 << $x}}
macro_rules! pop {($x:expr) => {$x &= $x - 1}}

/// Main position struct:
/// - Holds all information needed for the board state
/// - 6 piece bitboards and 2 colour bitboards
/// - Mailbox array for finding pieces quickly
/// - Incrementally updated zobrist hash, phase and endgame and midgame
/// piece-square table scores
pub struct Position {
    pub pieces: [u64; 6],
    pub sides: [u64; 2],
    pub squares: [u8; 64],
    pub c: bool,
    pub state: State,
    pub phase: i16,
    pub nulls: u8,
    pub castle: [u8; 2],
    pub chess960: bool,
    pub castle_mask: [u8; 64],
    pub stack: Vec<MoveContext>,
}

#[derive(Clone, Copy, Default)]
pub struct State {
    pub zobrist: u64,
    pub mg: i16,
    pub eg: i16,
    pub en_passant_sq: u16,
    pub halfmove_clock: u8,
    pub castle_rights: u8,
}

#[derive(Clone, Copy)]
pub struct MoveContext {
    state: State,
    m: u16,
    moved_pc: u8,
    captured_pc: u8,
}

impl Position {
    #[inline(always)]
    pub fn is_square_attacked(&self, idx: usize, side: usize, occ: u64) -> bool {
        let s: u64 = self.sides[side ^ 1];
        let opp_queen: u64 = self.pieces[QUEEN] & s;
        (KNIGHT_ATTACKS[idx] & self.pieces[KNIGHT] & s > 0)
        || (KING_ATTACKS[idx] & self.pieces[KING] & s > 0)
        || (PAWN_ATTACKS[side][idx] & self.pieces[PAWN] & s > 0)
        || (rook_attacks(idx, occ) & (self.pieces[ROOK] & s | opp_queen) > 0)
        || (bishop_attacks(idx, occ) & (self.pieces[BISHOP] & s | opp_queen) > 0)
    }

    pub fn is_in_check(&self) -> bool {
        let king_idx: usize = lsb!(self.pieces[KING] & self.sides[usize::from(self.c)]) as usize;
        self.is_square_attacked(king_idx, usize::from(self.c), self.sides[0] | self.sides[1])
    }

    #[inline(always)]
    fn toggle(&mut self, side: usize, piece: usize, bit: u64) {
        self.pieces[piece] ^= bit;
        self.sides[side] ^= bit;
    }

    #[inline(always)]
    fn add(&mut self, from: usize, side: usize, piece: usize) {
        let indx = from ^ (56 * (side == 0) as usize);
        self.state.zobrist ^= ZVALS.pieces[side][piece][from];
        self.state.mg += SIDE_FACTOR[side] * PST_MG[piece][indx];
        self.state.eg += SIDE_FACTOR[side] * PST_EG[piece][indx];
    }

    #[inline(always)]
    fn remove(&mut self, from: usize, side: usize, piece: usize) {
        let indx = from ^ (56 * (side == 0) as usize);
        self.state.zobrist ^= ZVALS.pieces[side][piece][from];
        self.state.mg -= SIDE_FACTOR[side] * PST_MG[piece][indx];
        self.state.eg -= SIDE_FACTOR[side] * PST_EG[piece][indx];
    }

    pub fn do_move(&mut self, m: u16) -> bool {
        let from: usize = from!(m);
        let to: usize = to!(m);
        let f: u64 = bit!(from);
        let t: u64 = bit!(to);
        let moved_pc: u8 = self.squares[from];
        let mpc: usize = moved_pc as usize;
        let captured_pc: u8 = self.squares[to];
        let flag: u16 = m & MoveFlags::ALL;
        let rights: u8 = self.state.castle_rights;
        let side: usize = usize::from(self.c);

        self.stack.push(MoveContext { state: self.state, m, moved_pc, captured_pc});
        self.toggle(side, mpc, f ^ t);
        self.remove(from, side, mpc);
        self.add(to, side, mpc);
        self.squares[from] = EMPTY as u8;
        self.squares[to] = moved_pc;
        if self.state.en_passant_sq > 0 {self.state.zobrist ^= ZVALS.en_passant[(self.state.en_passant_sq & 7) as usize]}
        self.state.en_passant_sq = 0;
        self.state.zobrist ^= ZVALS.side;
        if captured_pc != EMPTY as u8 && flag != MoveFlags::KS_CASTLE && flag != MoveFlags::QS_CASTLE {
            let cpc: usize = captured_pc as usize;
            self.toggle(side ^ 1, cpc, t);
            self.remove(to, side ^ 1, cpc);
            self.phase -= PHASE_VALS[cpc];
        }
        self.state.castle_rights &= self.castle_mask[from] & self.castle_mask[to];
        match flag {
            MoveFlags::EN_PASSANT => {
                let pwn: usize = if side == BLACK {to + 8} else {to - 8};
                let p: u64 = bit!(pwn);
                self.toggle(side ^ 1, PAWN, p);
                self.remove(pwn, side ^ 1, PAWN);
                self.squares[pwn] = EMPTY as u8;
            }
            MoveFlags::DBL_PUSH => {
                self.state.en_passant_sq = if side == WHITE {to - 8} else {to + 8} as u16;
                self.state.zobrist ^= ZVALS.en_passant[to & 7];
            }
            MoveFlags::KS_CASTLE | MoveFlags::QS_CASTLE => {
                let i: usize = (flag == MoveFlags::KS_CASTLE) as usize;
                let sq: usize = 56 * usize::from(side == BLACK) + self.castle[i] as usize;
                let idx: usize = CASTLE_MOVES[side][i];
                self.toggle(side, ROOK, (1 << idx) ^ (1 << sq));
                self.remove(sq, side, ROOK);
                self.squares[sq] = if to == sq {KING as u8} else {EMPTY as u8};
                self.squares[idx] = ROOK as u8;
                self.add(idx, side, ROOK);
            }
            MoveFlags::KNIGHT_PROMO.. => {
                let ppc: usize = (((flag >> 12) & 3) + 1) as usize;
                self.pieces[mpc] ^= t;
                self.pieces[ppc] ^= t;
                self.squares[to] = ppc as u8;
                self.phase += PHASE_VALS[ppc];
                self.remove(to, side, mpc);
                self.add(to, side, ppc);
            }
            _ => {}
        }
        self.state.halfmove_clock = u8::from(moved_pc > PAWN as u8 && flag != MoveFlags::CAPTURE) * (self.state.halfmove_clock + 1);
        self.c = !self.c;

        let mut changed_castle: u8 = rights & !self.state.castle_rights;
        while changed_castle > 0 {
            self.state.zobrist ^= ZVALS.castle[lsb!(changed_castle) as usize];
            pop!(changed_castle);
        }

        let king_idx: usize = lsb!(self.pieces[KING] & self.sides[side]) as usize;
        let invalid: bool = self.is_square_attacked(king_idx, side, self.sides[0] | self.sides[1]);
        if invalid { self.undo_move() }
        invalid
    }

    pub fn undo_move(&mut self) {
        let state: MoveContext = self.stack.pop().unwrap();
        let from: usize = from!(state.m);
        let to: usize = to!(state.m);
        let f: u64 = bit!(from);
        let t: u64 = bit!(to);
        let flag: u16 = state.m & MoveFlags::ALL;
        self.c = !self.c;
        let side: usize = usize::from(self.c);

        self.state = state.state;
        self.toggle(side, state.moved_pc as usize, f ^ t);
        self.squares[from] = state.moved_pc;
        self.squares[to] = state.captured_pc;
        if state.captured_pc != EMPTY as u8 && flag != MoveFlags::KS_CASTLE && flag != MoveFlags::QS_CASTLE {
            let cpc: usize = state.captured_pc as usize;
            self.toggle(side ^ 1, cpc, t);
            self.phase += PHASE_VALS[cpc];
        }
        match flag {
            MoveFlags::EN_PASSANT => {
                let pwn: usize = if side == BLACK {to + 8} else {to - 8};
                self.toggle(side ^ 1, PAWN, bit!(pwn));
                self.squares[pwn] = PAWN as u8;
            }
            MoveFlags::KS_CASTLE | MoveFlags::QS_CASTLE => {
                let i: usize = (flag == MoveFlags::KS_CASTLE) as usize;
                let sq: usize = 56 * usize::from(side == BLACK) + self.castle[i] as usize;
                let idx: usize = CASTLE_MOVES[side][i];
                self.squares[idx] = if from == idx {KING as u8} else {EMPTY as u8};
                self.squares[sq] = ROOK as u8;
                self.toggle(side, ROOK, (1 << idx) ^ (1 << sq));
            }
            MoveFlags::KNIGHT_PROMO.. => {
                let ppc: usize = (((flag >> 12) & 3) + 1) as usize;
                self.pieces[state.moved_pc as usize] ^= t;
                self.pieces[ppc] ^= t;
                self.phase -= PHASE_VALS[ppc];
            }
            _ => {}
        }
    }

    pub fn do_null(&mut self) -> (u16, u64) {
        self.nulls += 1;
        let enp: u16 = self.state.en_passant_sq;
        let hash: u64 = self.state.zobrist;
        self.state.zobrist ^= u64::from(enp > 0) * ZVALS.en_passant[(enp & 7) as usize];
        self.state.en_passant_sq = 0;
        self.c = !self.c;
        self.state.zobrist ^= ZVALS.side;
        (enp, hash)
    }

    pub fn undo_null(&mut self, (enp, hash): (u16, u64)) {
        self.nulls -= 1;
        self.state.zobrist = hash;
        self.state.en_passant_sq = enp;
        self.c = !self.c;
    }

    pub fn fifty_draw(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    pub fn repetition_draw(&self, num: u8) -> bool {
        let l: usize = self.stack.len();
        if l < 6 || self.nulls > 0 { return false }
        let to: usize = l - 1;
        let from: usize = l.saturating_sub(self.state.halfmove_clock as usize);
        let mut repetitions_count: u8 = 1;
        for i in (from..to).rev().step_by(2) {
            if self.stack[i].state.zobrist == self.state.zobrist {
                repetitions_count += 1;
                if repetitions_count >= num { return true }
            }
        }
        false
    }

    pub fn material_draw(&self) -> bool {
        let pawns: u64 = self.pieces[PAWN];
        if pawns == 0 && self.phase <= 2 {
            if self.phase == 2 {
                let b: u64 = self.pieces[BISHOP];
                return b & self.sides[0] != b && b & self.sides[1] != b && (b & 0x55AA_55AA_55AA_55AA == b || b & 0xAA55_AA55_AA55_AA55 == b)
            }
            return true
        }
        false
    }

    pub fn mvv_lva(&self, m: u16) -> u16 {
        let moved_pc: usize = self.squares[from!(m)] as usize;
        let captured_pc: usize = self.squares[to!(m)] as usize;
        MVV_LVA[captured_pc][moved_pc]
    }
}
