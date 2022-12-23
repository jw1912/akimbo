// engine details
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

// macro for calculating tables (until const fn pointers are stable)
macro_rules! init {
    ($init:stmt, $idx:expr, $initial:expr, $func:expr) => {{
        let mut res = [$initial; 64];
        $init
        while $idx < 64 {
            res[$idx] = $func;
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
    pub const NONE: u8 = 0;
}

// for promotions / double pushes
pub const PENRANK: [u64; 2] = [0x00FF000000000000, 0x000000000000FF00];
pub const DBLRANK: [u64; 2] = [0x00000000FF000000, 0x000000FF00000000];

// A file and ~(H file)
pub const FILE: u64 = 0x0101010101010101;
pub const NOTH: u64 = !(FILE << 7);

// rook attacks on rank
pub const WEST: [u64; 64] = init!(let mut idx = 0, idx, 0, ((1 << idx) - 1) & (0xFF << (idx & 56)));

// pawn attacks
pub const PAWN_ATTACKS: [[u64; 64]; 2] = [
    init!(let mut idx = 0, idx, 0, (((1 << idx) & !FILE) << 7) | (((1 << idx) & NOTH) << 9)),
    init!(let mut idx = 0, idx, 0, (((1 << idx) & !FILE) >> 9) | (((1 << idx) & NOTH) >> 7)),
];

// knight attacks
pub const KNIGHT_ATTACKS: [u64; 64] = init!(let mut idx = 0, idx, 0, {
    let n = 1 << idx;
    let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
    let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});

// king attacks
pub const KING_ATTACKS: [u64; 64] = init!(let mut idx = 0, idx, 0, {
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
pub const BMASKS: [Mask; 64] = init!(let mut idx = 0, idx, Mask { bit: 0, right: 0, left: 0, file: 0 }, {
    let bit = 1 << idx;
    Mask { bit, right: bit ^ DIAGS[(7 + (idx & 7) - (idx >> 3))], left: bit ^ DIAGS[((idx & 7) + (idx >> 3))].swap_bytes(), file: bit.swap_bytes() }
});

pub const RMASKS: [Mask; 64] = init!(let mut idx = 0, idx, Mask { bit: 0, right: 0, left: 0, file: 0 }, {
    let bit = 1 << idx;
    let left = (bit - 1) & (0xFF << (idx & 56));
    Mask { bit, right: bit ^ left ^ (0xFF << (idx & 56)), left, file: bit ^ FILE << (idx & 7) }
});

// castling
pub const CASTLE_MOVES: [[(u64, usize, usize);2];2] = [[(9, 0, 3), (160, 7, 5)], [(0x0900000000000000, 56, 59), (0xA000000000000000, 63, 61)]];
pub const B1C1D1: u64 = 14;
pub const F1G1: u64 = 96;
pub const B8C8D8: u64 = 0x0E00000000000000;
pub const F8G8: u64 = 0x6000000000000000;
pub const CASTLE_RIGHTS: [u8; 64] = init!(let mut idx = 0, idx, 0, match idx {0 => 7, 4 => 3, 7 => 11, 56 => 13, 60 => 12, 63 => 14, _ => 15});

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
    [82, 82, 82, 82, 82, 82, 82, 82, 191, 221, 168, 198, 180, 208, 106, 61, 76, 95, 115, 128, 143, 136, 117, 72, 64, 90, 83, 97, 100, 88, 95, 61, 56, 78, 79, 89, 92, 86, 95, 57, 57, 76, 74, 73, 83, 82, 117, 73, 49, 75, 62, 52, 65, 99, 124, 62, 82, 82, 82, 82, 82, 82, 82, 82],
    [171, 244, 305, 307, 397, 247, 329, 225, 283, 306, 395, 385, 369, 409, 346, 330, 308, 389, 384, 400, 437, 466, 407, 391, 349, 352, 368, 385, 372, 397, 361, 373, 331, 347, 358, 350, 362, 352, 364, 344, 314, 337, 343, 343, 361, 351, 355, 319, 303, 292, 327, 341, 335, 347, 332, 327, 225, 316, 276, 303, 318, 334, 316, 295],
    [328, 373, 293, 334, 336, 333, 365, 362, 339, 378, 361, 362, 394, 411, 369, 331, 351, 391, 399, 411, 410, 447, 411, 386, 353, 374, 394, 405, 395, 411, 374, 365, 373, 385, 379, 387, 398, 373, 373, 384, 368, 388, 378, 378, 373, 379, 383, 369, 385, 375, 378, 363, 370, 379, 389, 369, 327, 364, 351, 344, 358, 350, 336, 337],
    [515, 524, 519, 540, 551, 497, 513, 527, 509, 512, 536, 545, 555, 552, 510, 533, 483, 504, 507, 523, 514, 536, 551, 510, 459, 476, 492, 506, 504, 509, 488, 469, 451, 458, 471, 480, 486, 469, 494, 458, 436, 461, 467, 465, 478, 479, 479, 455, 436, 462, 461, 468, 479, 479, 479, 404, 463, 470, 475, 486, 486, 475, 447, 452],
    [1011, 1025, 1052, 1048, 1085, 1080, 1089, 1066, 1007, 991, 1027, 1026, 1018, 1096, 1047, 1083, 1020, 1014, 1024, 1037, 1061, 1096, 1088, 1070, 1007, 1009, 1014, 1016, 1030, 1041, 1035, 1035, 1016, 1003, 1017, 1019, 1026, 1022, 1029, 1029, 1012, 1028, 1016, 1019, 1019, 1029, 1036, 1022, 989, 1024, 1032, 1025, 1027, 1034, 1013, 1023, 1026, 1011, 1011, 1030, 1010, 994, 981, 970],
    [-54, 45, 34, 17, -28, -30, 17, 2, 49, 16, -7, 8, 7, -3, -10, -41, 4, 23, 5, -2, -11, 15, 20, -17, -9, -12, -11, -24, -24, -20, -9, -41, -45, -1, -25, -48, -45, -41, -34, -56, 3, -4, -19, -49, -46, -32, -14, -31, 15, 0, -17, -67, -47, -22, 14, 17, -34, 23, 3, -69, 7, -42, 29, 23]
];
pub static PST_EG: [[i16; 64];6] = [
    [94, 94, 94, 94, 94, 94, 94, 94, 266, 258, 244, 216, 223, 225, 270, 284, 195, 188, 175, 158, 149, 142, 176, 180, 135, 124, 111, 99, 92, 99, 115, 115, 116, 110, 94, 88, 89, 90, 101, 99, 105, 106, 92, 100, 95, 93, 94, 91, 115, 110, 105, 105, 110, 96, 96, 92, 94, 94, 94, 94, 94, 94, 94, 94],
    [238, 251, 272, 260, 247, 260, 217, 181, 259, 284, 258, 283, 273, 249, 253, 239, 264, 263, 293, 288, 273, 266, 263, 240, 262, 288, 304, 299, 300, 293, 291, 263, 261, 276, 296, 304, 297, 302, 282, 258, 249, 272, 277, 294, 285, 273, 260, 254, 239, 263, 264, 270, 275, 258, 254, 237, 250, 221, 266, 263, 255, 247, 231, 215],
    [285, 278, 297, 292, 294, 293, 273, 273, 291, 300, 310, 296, 295, 281, 289, 279, 300, 294, 298, 296, 296, 297, 299, 296, 291, 308, 303, 313, 310, 301, 309, 295, 287, 296, 312, 316, 302, 307, 295, 285, 284, 291, 303, 307, 308, 293, 287, 282, 274, 276, 286, 298, 299, 284, 277, 260, 274, 283, 266, 287, 283, 276, 281, 277],
    [520, 521, 528, 528, 521, 532, 528, 520, 525, 527, 526, 523, 510, 511, 524, 515, 528, 526, 527, 522, 520, 509, 506, 512, 524, 525, 529, 517, 514, 515, 517, 523, 516, 524, 524, 519, 509, 508, 504, 509, 511, 514, 509, 511, 501, 493, 504, 500, 509, 507, 513, 512, 501, 498, 503, 515, 505, 512, 517, 509, 503, 497, 519, 494],
    [937, 967, 961, 966, 957, 949, 945, 956, 927, 967, 977, 989, 1000, 955, 962, 932, 928, 951, 958, 984, 984, 975, 965, 940, 936, 965, 975, 989, 994, 977, 995, 978, 920, 964, 956, 985, 970, 975, 975, 959, 913, 910, 954, 939, 943, 946, 941, 937, 919, 911, 901, 920, 915, 906, 901, 904, 895, 905, 910, 888, 922, 894, 906, 890],
    [-56, -24, -17, -3, 16, 30, 20, -22, -10, 31, 33, 30, 28, 47, 52, 20, 12, 30, 35, 31, 36, 50, 54, 28, 0, 30, 37, 39, 37, 43, 36, 14, -7, 4, 27, 33, 34, 31, 18, -3, -23, 0, 12, 25, 26, 17, 6, -8, -27, -11, 6, 15, 15, 5, -9, -22, -46, -36, -18, -8, -36, -10, -31, -54]
];

// fen strings
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
pub const LASKER: &str = "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1";
pub const POSITIONS: [(&str, u8, u64); 5] = [
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 6, 119_060_324),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 5, 193_690_690),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -", 7, 178_633_661),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 5, 89_941_194),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 5, 164_075_551),
];

// uci <-> u16
pub const TWELVE: u16 = 0b0000_1111_1111_1111;
