use crate::{position::Position, util::Flag};

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Move {
    from: u8,
    to: u8,
    flag: u8,
    pc: u8,
}

impl Move {
    pub const NULL: Self = Self {
        from: 0,
        to: 0,
        flag: 0,
        pc: 0,
    };

    pub fn from(&self) -> usize {
        usize::from(self.from)
    }

    pub fn to(&self) -> usize {
        usize::from(self.to)
    }

    pub fn moved_pc(&self) -> usize {
        usize::from(self.pc)
    }

    pub fn flag(&self) -> u8 {
        self.flag
    }

    pub fn bb_to(&self) -> u64 {
        1 << self.to
    }

    pub fn bb_from(&self) -> u64 {
        1 << self.from
    }

    pub fn is_capture(&self) -> bool {
        self.flag & Flag::CAP > 0
    }

    pub fn is_noisy(&self) -> bool {
        self.flag >= Flag::CAP
    }

    pub fn is_en_passant(&self) -> bool {
        self.flag == Flag::ENP
    }

    pub fn is_promo(&self) -> bool {
        self.flag & Flag::PROMO > 0
    }

    pub fn promo_pc(&self) -> usize {
        usize::from(self.flag & 3) + 3
    }

    pub fn from_short(m: u16, pos: &Position) -> Self {
        let from = ((m >> 6) & 63) as u8;
        Self {
            from,
            to: (m & 63) as u8,
            flag: (m >> 12) as u8,
            pc: pos.get_pc(1 << from) as u8,
        }
    }

    pub fn to_short(&self) -> u16 {
        u16::from(self.from) << 6 | u16::from(self.to) | u16::from(self.flag) << 12
    }

    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {
            ["n","b","r","q"][(self.flag & 0b11) as usize]
        } else {
            ""
        };

        format!(
            "{}{}{}",
            idx_to_sq(self.from),
            idx_to_sq(self.to), promo
        )
    }
}

#[derive(Clone, Copy)]
pub struct MoveList {
    list: [Move; 252],
    len: usize,
}

impl std::ops::Deref for MoveList {
    type Target = [Move];
    fn deref(&self) -> &Self::Target {
        &self.list[..self.len]
    }
}

impl MoveList {
    pub const ZEROED: Self = Self {
        list: [Move::NULL; 252],
        len: 0,
    };

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn add(&mut self, mov: Move) {
        self.list[self.len] = mov;
        self.len += 1;
    }

    pub fn push(&mut self, from: u8, to: u8, flag: u8, mpc: usize) {
        self.list[self.len] = Move {
            from,
            to,
            flag,
            pc: mpc as u8,
        };
        self.len += 1;
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn pick(&mut self, scores: &mut [i32; 252]) -> Option<(Move, i32)> {
        if self.len == 0 {
            return None;
        }

        let mut idx = 0;
        let mut best = i32::MIN;

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

    pub fn copy_in(&mut self, mov:Move, other: &Self) {
        self.len = 1 + other.len;
        self.list[0] = mov;
        self.list[1..=other.len].copy_from_slice(&other.list[..other.len]);
    }
}