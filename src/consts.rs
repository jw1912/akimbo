use super::position::S;

// engine details
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

// fen strings
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// uci <-> u16
pub const TWELVE: u16 = 0b1111_1111_1111;

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

#[derive(Clone, Copy)]
pub struct Mask {
    pub bit: u64,
    pub right: u64,
    pub left: u64,
    pub file: u64,
}

// Types of move to be generated
pub const ALL: bool = true;
pub const CAPTURES: bool = false;

/// The type of bound determined by the hash entry when it was searched.
pub struct Bound;
impl Bound {
    pub const LOWER: u8 = 1;
    pub const UPPER: u8 = 2;
    pub const EXACT: u8 = 3;
}

// piece/side indices
pub const PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const ROOK: usize = 3;
pub const QUEEN: usize = 4;
pub const KING: usize = 5;
pub const EMPTY: usize = 6;
pub const WHITE: usize = 0;
pub const BLACK: usize = 1;

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
    pub const ROOK_PROMO: u16 = 10 << 12;
    pub const QUEEN_PROMO: u16 = 11 << 12;
    pub const KNIGHT_PROMO_CAPTURE: u16 = 12 << 12;
    pub const BISHOP_PROMO_CAPTURE: u16 = 13 << 12;
    pub const ROOK_PROMO_CAPTURE: u16 = 14 << 12;
    pub const QUEEN_PROMO_CAPTURE: u16 = 15 << 12;
}

pub struct CastleRights;
impl CastleRights {
    pub const WHITE_QS: u8 = 8;
    pub const WHITE_KS: u8 = 4;
    pub const BLACK_QS: u8 = 2;
    pub const BLACK_KS: u8 = 1;
    pub const SIDES: [u8; 2] = [Self::WHITE_KS | Self::WHITE_QS, Self::BLACK_KS | Self::BLACK_QS];
}

// for promotions / double pushes
pub const PENRANK: [u64; 2] = [0x00FF000000000000, 0x000000000000FF00];
pub const DBLRANK: [u64; 2] = [0x00000000FF000000, 0x000000FF00000000];

// A file and ~(H file)
pub const FILE: u64 = 0x0101010101010101;
pub const NOTH: u64 = !(FILE << 7);

// rook attacks on rank
pub const WEST: [u64; 64] = init!(idx, 0, ((1 << idx) - 1) & (0xFF << (idx & 56)));

// pawn attacks
pub const PAWN_ATTACKS: [[u64; 64]; 2] = [
    init!(idx, 0, (((1 << idx) & !FILE) << 7) | (((1 << idx) & NOTH) << 9)),
    init!(idx, 0, (((1 << idx) & !FILE) >> 9) | (((1 << idx) & NOTH) >> 7)),
];

// knight attacks
pub const KNIGHT_ATTACKS: [u64; 64] = init!(idx, 0, {
    let n = 1 << idx;
    let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
    let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});

// king attacks
pub const KING_ATTACKS: [u64; 64] = init!(idx, 0, {
    let mut k = 1 << idx;
    k |= (k << 8) | (k >> 8);
    k |= ((k & !FILE) >> 1) | ((k & NOTH) << 1);
    k ^ (1 << idx)
});

// diagonals
pub const DIAGS: [u64; 15] = [
    0x0100000000000000, 0x0201000000000000, 0x0402010000000000, 0x0804020100000000, 0x1008040201000000,
    0x2010080402010000, 0x4020100804020100, 0x8040201008040201, 0x0080402010080402, 0x0000804020100804,
    0x0000008040201008, 0x0000000080402010, 0x0000000000804020, 0x0000000000008040, 0x0000000000000080,
];

// masks for hyperbola quintessence rook and bishop attacks
pub const BMASKS: [Mask; 64] = init!(idx, Mask { bit: 0, right: 0, left: 0, file: 0 }, {
    let bit = 1 << idx;
    Mask { bit, right: bit ^ DIAGS[(7 + (idx & 7) - (idx >> 3))], left: bit ^ DIAGS[((idx & 7) + (idx >> 3))].swap_bytes(), file: bit.swap_bytes() }
});

pub const RMASKS: [Mask; 64] = init!(idx, Mask { bit: 0, right: 0, left: 0, file: 0 }, {
    let bit = 1 << idx;
    let left = (bit - 1) & (0xFF << (idx & 56));
    Mask { bit, right: bit ^ left ^ (0xFF << (idx & 56)), left, file: bit ^ FILE << (idx & 7) }
});

// castling
pub const CASTLE_MOVES: [[usize; 2]; 2] = [[3, 5], [59, 61]];

// search/eval
pub const MAX_PLY: i8 = 96;
pub const DELTA_MARGIN: i16 = 2 * MATERIAL[PAWN].1;
pub const KILLERS_PER_PLY: usize = 3;
pub const MAX: i16 = 30000;
pub const MATE_THRESHOLD: i16 = MAX - u8::MAX as i16;
pub const SIDE_FACTOR: [i16; 2] = [1, -1];
pub const PHASE_VALS: [i16; 7] = [0, 1, 1, 2, 4, 0, 0];
pub const TPHASE: i32 = 24;

// move ordering
pub const HASH_MOVE: u16 = 30000;
pub const PROMOTION: u16 = 600;
pub const KILLER: u16 = 500;
pub const QUIET: u16 = 0;
pub const MVV_LVA: [[u16; 7]; 7] = [
    [1500, 1400, 1300, 1200, 1100, 1000,    0],
    [2500, 2400, 2300, 2200, 2100, 2000,    0],
    [3500, 3400, 3300, 3200, 3100, 3000,    0],
    [4500, 4400, 4300, 4200, 4100, 4000,    0],
    [5500, 5400, 5300, 5200, 5100, 5000,    0],
    [   0,    0,    0,    0,    0,    0,    0],
    [   0,    0,    0,    0,    0,    0,    0],
];

// lazy eval values
pub const LAZY_MATERIAL: [S; 5] = [S(75, 113), S(318, 294), S(331, 308), S(450, 508), S(944, 945)];

// eval values
pub const MATERIAL: [S; 5] = [S(84, 142), S(315, 263), S(335, 276), S(437, 487), S(929, 933)];
pub const PAWN_PST: [S; 24] = [
    S( 69, 123), S( 92, 115), S( 85,  94), S( 93,  78),
    S(-27,  46), S( -1,  43), S( 19,  23), S( 34,  17),
    S(-35, -14), S(-15, -20), S(-21, -29), S( -7, -40),
    S(-35, -35), S(-19, -33), S(-16, -46), S(-10, -49),
    S(-26, -44), S( -4, -41), S(-20, -46), S(-16, -42),
    S(-33, -41), S(-11, -34), S(-22, -37), S(-31, -31),
];
pub const MOBILITY_KNIGHT: [S; 9] = [S(-35, -76), S(-7, -55), S(3, -33), S(8, -16), S(15, -4), S(18, 13), S(22, 17), S(22, 26), S(35, 14)];
pub const MOBILITY_BISHOP: [S; 14] = [S(-17, -73), S(-3, -57), S(4, -33), S(9, -17), S(13, -5), S(14, 6), S(16, 14), S(16, 16), S(17, 23), S(19, 23), S(35, 23), S(35, 24), S(36, 30), S(34, 30)];
pub const MOBILITY_ROOK: [S; 15] = [S(-31, -83), S(-14, -57), S(-17, -34), S(-16, -20), S(-11, -9), S(-11, -1), S(-8, 4), S(-5, 8), S(0, 15), S(8, 16), S(12, 19), S(14, 22), S(14, 25), S(16, 26), S(19, 27)];
pub const PAWN_SHIELD: S = S(22, -3);
pub const KING_LINEAR: S = S(-9, 0);
pub const KING_QUADRATIC: S = S(-6, 3);

pub const PST_IDX: [u8; 64] = init!(idx, 0, (((idx / 8) * 4).saturating_sub((4 - ((idx & 7) > 3) as i16 - (idx & 7) as i16).unsigned_abs() as usize)) as u8);
