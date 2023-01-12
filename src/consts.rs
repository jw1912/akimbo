// macro for calculating tables (until const fn pointers are stable)
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
pub const MAX_PLY: i8 = 96;
pub const KILLERS_PER_PLY: usize = 3;
pub const MAX: i16 = 30000;
pub const MATE_THRESHOLD: i16 = MAX - u8::MAX as i16;
pub const SIDE_FACTOR: [i16; 2] = [1, -1];
pub const PHASE_VALS: [i16; 7] = [0, 1, 1, 2, 4, 0, 0];
pub const TPHASE: i32 = 24;
pub const PST_IDX: [u8; 64] = init!(idx, 0, (((idx / 8) * 4).saturating_sub((4 - ((idx & 7) > 3) as i16 - (idx & 7) as i16).unsigned_abs() as usize)) as u8);

// Move Ordering
pub const HASH_MOVE: u16 = 30000;
pub const PROMOTION: u16 = 600;
pub const KILLER: u16 = 500;
pub const QUIET: u16 = 0;
pub const TVV: u16 = 40000;
pub const MVV_LVA: [[u16; 7]; 7] = [
    [1500, 1400, 1300, 1200, 1100, 1000,    0],
    [3450, 3350, 3250, 3150, 3050, 2950,    0],
    [3500, 3400, 3300, 3200, 3100, 3000,    0],
    [5500, 5400, 5300, 5200, 5100, 5000,    0],
    [8500, 8400, 8300, 8200, 8100, 8000,    0],
    [ TVV,  TVV,  TVV,  TVV,  TVV,  TVV,  TVV],
    [   0,    0,    0,    0,   0,    0,     0],
];

// Position
pub const   PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const   ROOK: usize = 3;
pub const  QUEEN: usize = 4;
pub const   KING: usize = 5;
pub const  EMPTY: usize = 6;
pub const  WHITE: usize = 0;
pub const  BLACK: usize = 1;
pub struct MoveFlags;
impl MoveFlags {
    pub const ALL: u16 = 15 << 12;
    pub const QUIET: u16 = 0 << 12;
    pub const DBL_PUSH: u16 = 1 << 12;
    pub const KS_CASTLE: u16 = 2 << 12;
    pub const QS_CASTLE: u16 = 3 << 12;
    pub const CAPTURE: u16 = 4 << 12;
    pub const EN_PASSANT: u16 = 5 << 12;
    pub const KNIGHT_PROMO: u16 = 8 << 12;
    pub const BISHOP_PROMO: u16 = 9 << 12;
    pub const   ROOK_PROMO: u16 = 10 << 12;
    pub const  QUEEN_PROMO: u16 = 11 << 12;
    pub const KNIGHT_PROMO_CAPTURE: u16 = 12 << 12;
    pub const BISHOP_PROMO_CAPTURE: u16 = 13 << 12;
    pub const   ROOK_PROMO_CAPTURE: u16 = 14 << 12;
    pub const  QUEEN_PROMO_CAPTURE: u16 = 15 << 12;
}
pub struct CastleRights;
impl CastleRights {
    pub const WHITE_QS: u8 = 8;
    pub const WHITE_KS: u8 = 4;
    pub const BLACK_QS: u8 = 2;
    pub const BLACK_KS: u8 = 1;
    pub const SIDES: [u8; 2] = [Self::WHITE_KS | Self::WHITE_QS, Self::BLACK_KS | Self::BLACK_QS];
}

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
