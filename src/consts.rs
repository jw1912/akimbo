// Creates a set of constants similar to in a C enum, but with a strict type and starts at a given value with an offset shift.
macro_rules! c_enum {
    ($type:ty, $val:expr, $offset:expr, $name:ident) => {pub const $name: $type = $val << $offset;};
    ($type:ty, $val:expr, $offset:expr, $name:ident, $($b:tt),*) => {pub const $name: $type = $val << $offset; c_enum!($type, $val + 1, $offset, $($b),*);}
}

// Macro for calculating tables (until const fn pointers are stable)
macro_rules! init {
    ($idx:ident, $init:expr, $($rest:tt)+) => {{
        let mut res = [$init; 64];
        let mut $idx = 0;
        while $idx < 64 {
            res[$idx] = {$($rest)+};
            $idx += 1;
        }
        res
    }};
}

// UCI
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const TWELVE: u16 = 0b1111_1111_1111;

// Search & Eval
pub const MAX_PLY: i16 = 96;
pub const KILLERS_PER_PLY: usize = 3;
pub const MAX: i16 = 30000;
pub const MATE: i16 = MAX - u8::MAX as i16;
pub const SIDE: [i16; 2] = [1, -1];
pub const PHASE_VALS: [i16; 7] = [0, 1, 1, 2, 4, 0, 0];
pub const TPHASE: i32 = 24;
pub const PAWN_IDX: [u8; 64] = init!(idx, 0, {
    let file = (idx & 7) as i8;
    (((idx / 8) * 4).saturating_sub((4 - (file > 3) as i8 - file).unsigned_abs() as usize)) as u8
});
pub const QT_IDX: [u8; 64] = init!(idx, 0, {
    let row = (idx / 8) as i8;
    let col = (idx & 7) as i8;
    ((row - 4 + (row > 3) as i8).abs() * 4 - (4 - col).abs() - (col > 3) as i8) as u8
});

// Move Ordering
pub const HASH_MOVE: u16 = 30000;
pub const PROMOTION: u16 = 950;
pub const KILLER: u16 = 900;
pub const MVV_LVA: [[u16; 6]; 5] = [
    [1500, 1400, 1300, 1200, 1100, 1000],
    [2500, 2400, 2300, 2200, 2100, 2000],
    [3500, 3400, 3300, 3200, 3100, 3000],
    [4500, 4400, 4300, 4200, 4100, 4000],
    [5500, 5400, 5300, 5200, 5100, 5000],
];

// Position
pub const WHITE: usize = 0;
pub const BLACK: usize = 1;
c_enum!(usize, 0, 0, PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING, EMPTY);
pub const ALL_FLAGS: u16 = 15 << 12;
c_enum!(u16, 0, 12, QUIET, DBL, KS, QS, CAP, ENP, _A, _B, PR, BPR, RPR, QPR, NPC, BPC, RPC, QPC);
pub const WQS: u8 = 8;
pub const WKS: u8 = 4;
pub const BQS: u8 = 2;
pub const BKS: u8 = 1;
pub const CS: [u8; 2] = [WKS | WQS, BKS | BQS];

// Move Generation
#[derive(Clone, Copy)]
pub struct Mask {
    pub bit: u64,
    pub right: u64,
    pub left: u64,
    pub file: u64,
}
pub const ALL: bool = true;
pub const CAPTURES: bool = false;
pub const PENRANK: [u64; 2] = [0x00FF000000000000, 0x000000000000FF00];
pub const DBLRANK: [u64; 2] = [0x00000000FF000000, 0x000000FF00000000];
pub const FILE: u64 = 0x0101010101010101;
pub const NOTH: u64 = !(FILE << 7);
pub const WEST: [u64; 64] = init!(idx, 0, ((1 << idx) - 1) & (0xFF << (idx & 56)));
pub const PAWN_ATTACKS: [[u64; 64]; 2] = [
    init!(idx, 0, (((1 << idx) & !FILE) << 7) | (((1 << idx) & NOTH) << 9)),
    init!(idx, 0, (((1 << idx) & !FILE) >> 9) | (((1 << idx) & NOTH) >> 7)),
];
pub const KNIGHT_ATTACKS: [u64; 64] = init!(idx, 0, {
    let n = 1 << idx;
    let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
    let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});
pub const KING_ATTACKS: [u64; 64] = init!(idx, 0, {
    let mut k = 1 << idx;
    k |= (k << 8) | (k >> 8);
    k |= ((k & !FILE) >> 1) | ((k & NOTH) << 1);
    k ^ (1 << idx)
});
pub const DIAGS: [u64; 15] = [
    0x0100000000000000, 0x0201000000000000, 0x0402010000000000, 0x0804020100000000, 0x1008040201000000,
    0x2010080402010000, 0x4020100804020100, 0x8040201008040201, 0x0080402010080402, 0x0000804020100804,
    0x0000008040201008, 0x0000000080402010, 0x0000000000804020, 0x0000000000008040, 0x0000000000000080,
];
pub const BMASKS: [Mask; 64] = init!(idx, Mask { bit: 0, right: 0, left: 0, file: 0 }, {
    let bit = 1 << idx;
    Mask { bit, right: bit ^ DIAGS[(7 + (idx & 7) - (idx >> 3))], left: bit ^ DIAGS[((idx & 7) + (idx >> 3))].swap_bytes(), file: bit.swap_bytes() }
});
pub const RMASKS: [Mask; 64] = init!(idx, Mask { bit: 0, right: 0, left: 0, file: 0 }, {
    let bit = 1 << idx;
    let left = (bit - 1) & (0xFF << (idx & 56));
    Mask { bit, right: bit ^ left ^ (0xFF << (idx & 56)), left, file: bit ^ FILE << (idx & 7) }
});
pub const CASTLE_MOVES: [[usize; 2]; 2] = [[3, 5], [59, 61]];

// Zobrist Hashing Values
pub static ZVALS: ZobristVals = {
    let mut seed: u64 = 180_620_142;
    seed = xor_shift(seed);
    let mut vals: ZobristVals = ZobristVals { pieces: [[[0; 64]; 6]; 2], castle: [0; 4], en_passant: [0; 8], side: seed };
    let mut idx: usize = 0;
    while idx < 2 {
        let mut piece: usize = 0;
        while piece < 6 {
            let mut square: usize = 0;
            while square < 64 {
                seed = xor_shift(seed);
                vals.pieces[idx][piece][square] = seed;
                square += 1;
            } piece += 1;
        } idx += 1;
    }
    while idx < 6 {seed = xor_shift(seed); vals.castle[idx - 2] = seed; idx += 1;}
    while idx < 14 {seed = xor_shift(seed); vals.en_passant[idx - 6] = seed; idx += 1;}
    vals
};

pub struct ZobristVals {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub castle: [u64; 4],
    pub en_passant: [u64; 8],
    pub side: u64,
}

const fn xor_shift(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}
