use super::position::S;

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
pub const MAX_PLY: i8 = 96;
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
pub static PST: [[S; 64]; 6] = [
    [S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(234, 248), S(247, 246), S(208, 224), S(212, 207), S(213, 211), S(217, 217), S(147, 256), S(92, 273), S(86, 189), S(105, 182), S(127, 168), S(142, 147), S(151, 140), S(144, 136), S(131, 169), S(85, 170), S(69, 132), S(91, 121), S(84, 108), S(98, 95), S(103, 86), S(91, 94), S(100, 109), S(69, 108), S(60, 112), S(78, 107), S(79, 91), S(90, 84), S(92, 85), S(85, 87), S(99, 95), S(64, 91), S(62, 100), S(77, 102), S(74, 89), S(72, 97), S(82, 92), S(81, 89), S(122, 88), S(80, 84), S(54, 109), S(75, 107), S(62, 100), S(51, 102), S(64, 105), S(98, 93), S(126, 92), S(68, 85), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100)],
    [S(132, 240), S(217, 250), S(265, 276), S(294, 258), S(342, 256), S(222, 259), S(263, 228), S(175, 187), S(241, 265), S(266, 289), S(348, 267), S(353, 284), S(331, 277), S(373, 254), S(304, 257), S(284, 249), S(266, 270), S(346, 271), S(344, 298), S(359, 293), S(404, 275), S(414, 275), S(362, 272), S(348, 247), S(308, 270), S(309, 296), S(329, 308), S(342, 307), S(329, 308), S(358, 298), S(319, 298), S(333, 267), S(287, 266), S(303, 283), S(315, 303), S(307, 310), S(320, 304), S(310, 305), S(324, 286), S(302, 262), S(271, 253), S(296, 274), S(299, 284), S(299, 299), S(318, 291), S(308, 279), S(312, 267), S(276, 260), S(261, 244), S(249, 266), S(286, 265), S(298, 275), S(291, 281), S(307, 258), S(291, 259), S(287, 243), S(207, 239), S(272, 226), S(240, 267), S(263, 265), S(277, 259), S(296, 248), S(271, 240), S(246, 222)],
    [S(260, 298), S(333, 282), S(255, 302), S(295, 295), S(270, 304), S(294, 296), S(307, 283), S(303, 279), S(291, 299), S(334, 307), S(323, 313), S(322, 300), S(356, 297), S(361, 291), S(323, 299), S(282, 285), S(306, 306), S(339, 304), S(354, 305), S(366, 305), S(361, 307), S(419, 297), S(368, 305), S(344, 299), S(307, 299), S(326, 317), S(356, 307), S(362, 318), S(352, 315), S(368, 308), S(329, 317), S(319, 303), S(330, 291), S(342, 302), S(333, 319), S(347, 319), S(351, 310), S(327, 315), S(330, 300), S(342, 286), S(324, 292), S(343, 298), S(333, 309), S(331, 315), S(327, 317), S(329, 306), S(339, 295), S(323, 290), S(342, 274), S(328, 286), S(333, 290), S(316, 308), S(322, 309), S(335, 289), S(339, 294), S(325, 265), S(288, 274), S(322, 284), S(304, 272), S(300, 293), S(315, 286), S(301, 287), S(303, 281), S(295, 278)],
    [S(508, 504), S(519, 504), S(532, 504), S(539, 507), S(551, 501), S(513, 507), S(523, 504), S(529, 500), S(503, 509), S(509, 509), S(532, 509), S(540, 506), S(540, 500), S(547, 496), S(524, 502), S(534, 499), S(486, 507), S(506, 507), S(507, 508), S(527, 500), S(525, 496), S(539, 492), S(554, 488), S(518, 491), S(458, 505), S(479, 504), S(494, 508), S(505, 499), S(500, 498), S(510, 496), S(496, 496), S(480, 500), S(455, 494), S(459, 502), S(468, 505), S(480, 497), S(483, 491), S(463, 491), S(498, 483), S(463, 489), S(438, 488), S(459, 492), S(469, 486), S(465, 491), S(477, 480), S(479, 471), S(483, 479), S(461, 476), S(436, 487), S(460, 487), S(462, 493), S(469, 489), S(478, 480), S(477, 477), S(482, 478), S(407, 493), S(466, 476), S(474, 482), S(477, 494), S(486, 489), S(485, 481), S(474, 479), S(454, 490), S(458, 466)],
    [S(882, 923), S(899, 953), S(919, 952), S(937, 940), S(958, 943), S(968, 932), S(968, 928), S(934, 944), S(878, 917), S(868, 952), S(905, 962), S(905, 973), S(905, 975), S(968, 944), S(921, 954), S(951, 922), S(895, 916), S(887, 941), S(898, 948), S(918, 963), S(940, 969), S(980, 955), S(963, 946), S(942, 935), S(890, 907), S(885, 949), S(892, 960), S(897, 965), S(917, 965), S(919, 958), S(915, 967), S(916, 944), S(893, 904), S(885, 937), S(891, 940), S(897, 965), S(902, 952), S(899, 956), S(911, 941), S(909, 932), S(893, 886), S(905, 885), S(895, 928), S(894, 918), S(895, 919), S(909, 912), S(913, 921), S(899, 913), S(867, 903), S(901, 891), S(910, 874), S(901, 894), S(902, 894), S(913, 878), S(884, 891), S(902, 884), S(909, 864), S(892, 873), S(886, 892), S(906, 864), S(889, 894), S(873, 872), S(852, 895), S(850, 869)],
    [S(-12, -57), S(78, -24), S(106, -29), S(49, -3), S(8, 14), S(14, 26), S(45, 23), S(15, -14), S(77, -9), S(30, 34), S(21, 32), S(68, 23), S(18, 31), S(19, 49), S(11, 56), S(-40, 28), S(43, 10), S(29, 34), S(16, 38), S(17, 32), S(8, 37), S(50, 48), S(41, 57), S(-6, 32), S(11, 1), S(9, 30), S(-2, 39), S(-15, 41), S(-18, 39), S(9, 42), S(13, 38), S(-51, 21), S(-38, -4), S(22, 2), S(1, 25), S(-36, 34), S(-26, 34), S(-18, 30), S(-27, 21), S(-53, 2), S(17, -23), S(13, -2), S(-16, 14), S(-33, 24), S(-34, 26), S(-29, 19), S(0, 5), S(-32, -3), S(26, -29), S(-6, -4), S(-18, 8), S(-65, 17), S(-45, 17), S(-19, 7), S(14, -6), S(26, -24), S(-44, -38), S(19, -28), S(2, -12), S(-72, -5), S(4, -33), S(-46, -5), S(33, -30), S(25, -52)],
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

pub const FRC_POSITIONS: [(&str, u8, u64); 5] = [
    ("bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9", 5, 8146062),
    ("2nnrbkr/p1qppppp/8/1ppb4/6PP/3PP3/PPP2P2/BQNNRBKR w HEhe - 1 9", 5, 16253601),
    ("b1q1rrkb/pppppppp/3nn3/8/P7/1PPP4/4PPPP/BQNNRKRB w GE - 1 9", 5, 6417013),
    ("qbbnnrkr/2pp2pp/p7/1p2pp2/8/P3PP2/1PPP1KPP/QBBNNR1R w hf - 0 9", 5, 9183776),
    ("1nbbnrkr/p1p1ppp1/3p4/1p3P1p/3Pq2P/8/PPP1P1P1/QNBBNRKR w HFhf - 0 9", 5, 34030312)
];
