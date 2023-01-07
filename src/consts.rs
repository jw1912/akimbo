// engine details
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

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
pub const MAX_PLY: i8 = i8::MAX - 8;
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
pub const MVV_LVA: [[u16; 7]; 7] = [[1500, 1400, 1300, 1200, 1100, 1000, 0], [2500, 2400, 2300, 2200, 2100, 2000, 0], [3500, 3400, 3300, 3200, 3100, 3000, 0], [4500, 4400, 4300, 4200, 4100, 4000, 0], [5500, 5400, 5300, 5200, 5100, 5000,0], [0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0, 0]];

// eval values
pub static PST_MG: [[i16; 64];6] = [
    [100, 100, 100, 100, 100, 100, 100, 100, 234, 247, 208, 212, 213, 217, 147,  92, 86, 105, 127, 142, 151, 144, 131,  85, 69,  91,  84,  98, 103,  91, 100,  69, 60, 78, 79, 90, 92, 85, 99, 64, 62, 77, 74, 72, 82, 81, 122, 80, 54, 75, 62, 51, 64, 98, 126, 68, 100, 100, 100, 100, 100, 100, 100, 100,],
    [132, 217, 265, 294, 342, 222, 263, 175, 241, 266, 348, 353, 331, 373, 304, 284, 266, 346, 344, 359, 404, 414, 362, 348, 308, 309, 329, 342, 329, 358, 319, 333, 287, 303, 315, 307, 320, 310, 324, 302, 271, 296, 299, 299, 318, 308, 312, 276, 261, 249, 286, 298, 291, 307, 291, 287, 207, 272, 240, 263, 277, 296, 271, 246],
    [260, 333, 255, 295, 270, 294, 307, 303, 291, 334, 323, 322, 356, 361, 323, 282, 306, 339, 354, 366, 361, 419, 368, 344, 307, 326, 356, 362, 352, 368, 329, 319, 330, 342, 333, 347, 351, 327, 330, 342, 324, 343, 333, 331, 327, 329, 339, 323, 342, 328, 333, 316, 322, 335, 339, 325, 288, 322, 304, 300, 315, 301, 303, 295],
    [508, 519, 532, 539, 551, 513, 523, 529, 503, 509, 532, 540, 540, 547, 524, 534, 486, 506, 507, 527, 525, 539, 554, 518, 458, 479, 494, 505, 500, 510, 496, 480, 455, 459, 468, 480, 483, 463, 498, 463, 438, 459, 469, 465, 477, 479, 483, 461, 436, 460, 462, 469, 478, 477, 482, 407, 466, 474, 477, 486, 485, 474, 454, 458],
    [882, 899, 919, 937, 958, 968, 968, 934, 878, 868, 905, 905, 905, 968, 921, 951, 895, 887, 898, 918, 940, 980, 963, 942, 890, 885, 892, 897, 917, 919, 915, 916, 893, 885, 891, 897, 902, 899, 911, 909, 893, 905, 895, 894, 895, 909, 913, 899, 867, 901, 910, 901, 902, 913, 884, 902, 909, 892, 886, 906, 889, 873, 852, 850],
    [-12, 78, 106, 49, 8, 14, 45, 15, 77, 30, 21, 68, 18, 19, 11, -40, 43, 29, 16, 17, 8, 50, 41, -6, 11, 9, -2, -15, -18, 9, 13, -51, -38, 22, 1, -36, -26, -18, -27, -53, 17, 13, -16, -33, -34, -29, 0, -32, 26, -6, -18, -65, -45, -19, 14, 26, -44, 19, 2, -72, 4, -46, 33, 25]
];
pub static PST_EG: [[i16; 64];6] = [
    [100, 100, 100, 100, 100, 100, 100, 100, 248, 246, 224, 207, 211, 217, 256, 273, 189, 182, 168, 147, 140, 136, 169, 170, 132, 121, 108, 95, 86, 94, 109, 108, 112, 107, 91, 84, 85, 87, 95, 91, 100, 102, 89, 97, 92, 89, 88, 84, 109, 107, 100, 102, 105, 93, 92, 85, 100, 100, 100, 100, 100, 100, 100, 100],
    [240, 250, 276, 258, 256, 259, 228, 187, 265, 289, 267, 284, 277, 254, 257, 249, 270, 271, 298, 293, 275, 275, 272, 247, 270, 296, 308, 307, 308, 298, 298, 267, 266, 283, 303, 310, 304, 305, 286, 262, 253, 274, 284, 299, 291, 279, 267, 260, 244, 266, 265, 275, 281, 258, 259, 243, 239, 226, 267, 265, 259, 248, 240, 222],
    [298, 282, 302, 295, 304, 296, 283, 279, 299, 307, 313, 300, 297, 291, 299, 285, 306, 304, 305, 305, 307, 297, 305, 299, 299, 317, 307, 318, 315, 308, 317, 303, 291, 302, 319, 319, 310, 315, 300, 286, 292, 298, 309, 315, 317, 306, 295, 290, 274, 286, 290, 308, 309, 289, 294, 265, 274, 284, 272, 293, 286, 287, 281, 278],
    [504, 504, 504, 507, 501, 507, 504, 500, 509, 509, 509, 506, 500, 496, 502, 499, 507, 507, 508, 500, 496, 492, 488, 491, 505, 504, 508, 499, 498, 496, 496, 500, 494, 502, 505, 497, 491, 491, 483, 489, 488, 492, 486, 491, 480, 471, 479, 476, 487, 487, 493, 489, 480, 477, 478, 493, 476, 482, 494, 489, 481, 479, 490, 466],
    [923, 953, 952, 940, 943, 932, 928, 944, 917, 952, 962, 973, 975, 944, 954, 922, 916, 941, 948, 963, 969, 955, 946, 935, 907, 949, 960, 965, 965, 958, 967, 944, 904, 937, 940, 965, 952, 956, 941, 932, 886, 885, 928, 918, 919, 912, 921, 913, 903, 891, 874, 894, 894, 878, 891, 884, 864, 873, 892, 864, 894, 872, 895, 869],
    [-57, -24, -29, -3, 14, 26, 23, -14, -9, 34, 32, 23, 31, 49, 56, 28, 10, 34, 38, 32, 37, 48, 57, 32, 1, 30, 39, 41, 39, 42, 38, 21, -4, 2, 25, 34, 34, 30, 21, 2, -23, -2, 14, 24, 26, 19, 5, -3, -29, -4, 8, 17, 17, 7, -6, -24, -38, -28, -12, -5, -33, -5, -30, -52]
];

// fen strings
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// uci <-> u16
pub const TWELVE: u16 = 0b1111_1111_1111;

pub const POSITIONS: [(&str, u8, u64); 5] = [
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 6, 119_060_324),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 5, 193_690_690),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -", 7, 178_633_661),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 5, 89_941_194),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 5, 164_075_551),
];

pub const FRC_POSITIONS: [(&str, u8, u64); 2] = [
    ("bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9", 5, 8146062),
    ("2nnrbkr/p1qppppp/8/1ppb4/6PP/3PP3/PPP2P2/BQNNRBKR w HEhe - 1 9", 5, 16253601),
];
