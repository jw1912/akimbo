use std::ops::{AddAssign, Mul};

// UCI
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
// Search
pub const MAX_PLY: i16 = 96;
pub const KILLERS: usize = 3;
pub const MAX: i16 = 30000;
pub const MATE: i16 = MAX - u8::MAX as i16;
pub const LOWER: u8 = 0b0100_0000;
pub const EXACT: u8 = 0b1100_0000;
pub const UPPER: u8 = 0b1000_0000;
// Eval
pub const SIDE: [i16; 2] = [1, -1];
pub const PHASE_VALS: [i16; 8] = [0, 0, 0, 1, 1, 2, 4, 0];
pub const TPHASE: i32 = 24;
// Move Ordering
pub const HASH_MOVE: i16 = 30000;
pub const PROMOTION: i16 = 950;
pub const KILLER: i16 = 900;
pub const MVV_LVA: [[i16; 8]; 8] = [[0, 0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0, 0, 0], [0, 0, 1500, 1400, 1300, 1200, 1100, 1000], [0, 0, 2500, 2400, 2300, 2200, 2100, 2000], [0, 0, 3500, 3400, 3300, 3200, 3100, 3000], [0, 0, 4500, 4400, 4300, 4200, 4100, 4000], [0, 0, 5500, 5400, 5300, 5200, 5100, 5000], [0, 0, 0, 0, 0, 0, 0, 0]];
// Evaluation
#[derive(Clone, Copy, Debug, Default)]
pub struct S(pub i16, pub i16);
impl AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}
impl Mul<S> for i16 {
    type Output = S;
    fn mul(self, rhs: S) -> Self::Output {
        S(self * rhs.0, self * rhs.1)
    }
}
pub static PST: [[S; 64]; 8] = [
    [S(0, 0); 64], [S(0, 0); 64],
    [S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(234, 248), S(247, 246), S(208, 224), S(212, 207), S(213, 211), S(217, 217), S(147, 256), S(92, 273), S(86, 189), S(105, 182), S(127, 168), S(142, 147), S(151, 140), S(144, 136), S(131, 169), S(85, 170), S(69, 132), S(91, 121), S(84, 108), S(98, 95), S(103, 86), S(91, 94), S(100, 109), S(69, 108), S(60, 112), S(78, 107), S(79, 91), S(90, 84), S(92, 85), S(85, 87), S(99, 95), S(64, 91), S(62, 100), S(77, 102), S(74, 89), S(72, 97), S(82, 92), S(81, 89), S(122, 88), S(80, 84), S(54, 109), S(75, 107), S(62, 100), S(51, 102), S(64, 105), S(98, 93), S(126, 92), S(68, 85), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100)],
    [S(132, 240), S(217, 250), S(265, 276), S(294, 258), S(342, 256), S(222, 259), S(263, 228), S(175, 187), S(241, 265), S(266, 289), S(348, 267), S(353, 284), S(331, 277), S(373, 254), S(304, 257), S(284, 249), S(266, 270), S(346, 271), S(344, 298), S(359, 293), S(404, 275), S(414, 275), S(362, 272), S(348, 247), S(308, 270), S(309, 296), S(329, 308), S(342, 307), S(329, 308), S(358, 298), S(319, 298), S(333, 267), S(287, 266), S(303, 283), S(315, 303), S(307, 310), S(320, 304), S(310, 305), S(324, 286), S(302, 262), S(271, 253), S(296, 274), S(299, 284), S(299, 299), S(318, 291), S(308, 279), S(312, 267), S(276, 260), S(261, 244), S(249, 266), S(286, 265), S(298, 275), S(291, 281), S(307, 258), S(291, 259), S(287, 243), S(207, 239), S(272, 226), S(240, 267), S(263, 265), S(277, 259), S(296, 248), S(271, 240), S(246, 222)],
    [S(260, 298), S(333, 282), S(255, 302), S(295, 295), S(270, 304), S(294, 296), S(307, 283), S(303, 279), S(291, 299), S(334, 307), S(323, 313), S(322, 300), S(356, 297), S(361, 291), S(323, 299), S(282, 285), S(306, 306), S(339, 304), S(354, 305), S(366, 305), S(361, 307), S(419, 297), S(368, 305), S(344, 299), S(307, 299), S(326, 317), S(356, 307), S(362, 318), S(352, 315), S(368, 308), S(329, 317), S(319, 303), S(330, 291), S(342, 302), S(333, 319), S(347, 319), S(351, 310), S(327, 315), S(330, 300), S(342, 286), S(324, 292), S(343, 298), S(333, 309), S(331, 315), S(327, 317), S(329, 306), S(339, 295), S(323, 290), S(342, 274), S(328, 286), S(333, 290), S(316, 308), S(322, 309), S(335, 289), S(339, 294), S(325, 265), S(288, 274), S(322, 284), S(304, 272), S(300, 293), S(315, 286), S(301, 287), S(303, 281), S(295, 278)],
    [S(508, 504), S(519, 504), S(532, 504), S(539, 507), S(551, 501), S(513, 507), S(523, 504), S(529, 500), S(503, 509), S(509, 509), S(532, 509), S(540, 506), S(540, 500), S(547, 496), S(524, 502), S(534, 499), S(486, 507), S(506, 507), S(507, 508), S(527, 500), S(525, 496), S(539, 492), S(554, 488), S(518, 491), S(458, 505), S(479, 504), S(494, 508), S(505, 499), S(500, 498), S(510, 496), S(496, 496), S(480, 500), S(455, 494), S(459, 502), S(468, 505), S(480, 497), S(483, 491), S(463, 491), S(498, 483), S(463, 489), S(438, 488), S(459, 492), S(469, 486), S(465, 491), S(477, 480), S(479, 471), S(483, 479), S(461, 476), S(436, 487), S(460, 487), S(462, 493), S(469, 489), S(478, 480), S(477, 477), S(482, 478), S(407, 493), S(466, 476), S(474, 482), S(477, 494), S(486, 489), S(485, 481), S(474, 479), S(454, 490), S(458, 466)],
    [S(882, 923), S(899, 953), S(919, 952), S(937, 940), S(958, 943), S(968, 932), S(968, 928), S(934, 944), S(878, 917), S(868, 952), S(905, 962), S(905, 973), S(905, 975), S(968, 944), S(921, 954), S(951, 922), S(895, 916), S(887, 941), S(898, 948), S(918, 963), S(940, 969), S(980, 955), S(963, 946), S(942, 935), S(890, 907), S(885, 949), S(892, 960), S(897, 965), S(917, 965), S(919, 958), S(915, 967), S(916, 944), S(893, 904), S(885, 937), S(891, 940), S(897, 965), S(902, 952), S(899, 956), S(911, 941), S(909, 932), S(893, 886), S(905, 885), S(895, 928), S(894, 918), S(895, 919), S(909, 912), S(913, 921), S(899, 913), S(867, 903), S(901, 891), S(910, 874), S(901, 894), S(902, 894), S(913, 878), S(884, 891), S(902, 884), S(909, 864), S(892, 873), S(886, 892), S(906, 864), S(889, 894), S(873, 872), S(852, 895), S(850, 869)],
    [S(-12, -57), S(78, -24), S(106, -29), S(49, -3), S(8, 14), S(14, 26), S(45, 23), S(15, -14), S(77, -9), S(30, 34), S(21, 32), S(68, 23), S(18, 31), S(19, 49), S(11, 56), S(-40, 28), S(43, 10), S(29, 34), S(16, 38), S(17, 32), S(8, 37), S(50, 48), S(41, 57), S(-6, 32), S(11, 1), S(9, 30), S(-2, 39), S(-15, 41), S(-18, 39), S(9, 42), S(13, 38), S(-51, 21), S(-38, -4), S(22, 2), S(1, 25), S(-36, 34), S(-26, 34), S(-18, 30), S(-27, 21), S(-53, 2), S(17, -23), S(13, -2), S(-16, 14), S(-33, 24), S(-34, 26), S(-29, 19), S(0, 5), S(-32, -3), S(26, -29), S(-6, -4), S(-18, 8), S(-65, 17), S(-45, 17), S(-19, 7), S(14, -6), S(26, -24), S(-44, -38), S(19, -28), S(2, -12), S(-72, -5), S(4, -33), S(-46, -5), S(33, -30), S(25, -52)],
];
// Move generation
pub const ALL: bool = true;
pub const CAPTURES: bool = false;
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
macro_rules! c_enum {
    ($type:ty, $val:expr, $name:ident) => {pub const $name: $type = $val;};
    ($type:ty, $val:expr, $name:ident, $($b:tt),*) => {pub const $name: $type = $val; c_enum!($type, $val + 1, $($b),*);};
}
pub const E: usize = 0;
c_enum!(usize, 0, WH, BL, P, N, B, R, Q, K);
c_enum!(u8, 0, QUIET, DBL, KS, QS, CAP, ENP, _E1, _E2, NPR, _BPR, _RPR, QPR, NPC, _BPC, _RPC, QPC);
// Castling
pub const WQS: u8 = 0b1000;
pub const WKS: u8 = 0b0100;
pub const BQS: u8 = 0b0010;
pub const BKS: u8 = 0b0001;
pub const B1C1D1: u64 = 0x000000000000000E;
pub const   F1G1: u64 = 0x0000000000000060;
pub const B8C8D8: u64 = 0x0E00000000000000;
pub const   F8G8: u64 = 0x6000000000000000;
pub const CS: [u8; 2] = [WKS | WQS, BKS | BQS];
pub const CR: [u8; 64] = init!(idx, 0, match idx {0 => 7, 4 => 3, 7 => 11, 56 => 13, 60 => 12, 63 => 14, _ => 15});
pub const CM: [[(u64, usize, usize); 2]; 2] = [[(9, 0, 3), (0x0900000000000000, 56, 59)], [(160, 7, 5), (0xA000000000000000, 63, 61)]];
// Pawns
pub const PENRANK: [u64; 2] = [0x00FF000000000000, 0x000000000000FF00];
pub const DBLRANK: [u64; 2] = [0x00000000FF000000, 0x000000FF00000000];
pub const FILE: u64 = 0x0101010101010101;
pub const NOTH: u64 = !(FILE << 7);
pub const PATT: [[u64; 64]; 2] = [
    init!(idx, 0, (((1 << idx) & !FILE) << 7) | (((1 << idx) & NOTH) << 9)),
    init!(idx, 0, (((1 << idx) & !FILE) >> 9) | (((1 << idx) & NOTH) >> 7)),
];
// King and knight attacks
pub const NATT: [u64; 64] = init!(idx, 0, {
    let n = 1 << idx;
    let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
    let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});
pub const KATT: [u64; 64] = init!(idx, 0, {
    let mut k = 1 << idx;
    k |= (k << 8) | (k >> 8);
    k |= ((k & !FILE) >> 1) | ((k & NOTH) << 1);
    k ^ (1 << idx)
});
// Slider attacks
#[derive(Clone, Copy)]
pub struct Mask {
    pub bit: u64,
    pub right: u64,
    pub left: u64,
    pub file: u64,
}
pub const WEST: [u64; 64] = init!(idx, 0, ((1 << idx) - 1) & (0xFF << (idx & 56)));
pub const DIAGS: [u64; 15] = [
    0x0100000000000000, 0x0201000000000000, 0x0402010000000000, 0x0804020100000000, 0x1008040201000000,
    0x2010080402010000, 0x4020100804020100, 0x8040201008040201, 0x0080402010080402, 0x0000804020100804,
    0x0000008040201008, 0x0000000080402010, 0x0000000000804020, 0x0000000000008040, 0x0000000000000080,
];
pub const BMASKS: [Mask; 64] = init!(idx, Mask { bit: 0, right: 0, left: 0, file: 0 },
    let bit = 1 << idx;
    Mask { bit, right: bit ^ DIAGS[(7 + (idx & 7) - (idx >> 3))], left: bit ^ DIAGS[((idx & 7) + (idx >> 3))].swap_bytes(), file: bit.swap_bytes() }
);
pub const RMASKS: [Mask; 64] = init!(idx, Mask { bit: 0, right: 0, left: 0, file: 0 },
    let bit = 1 << idx;
    let left = (bit - 1) & (0xFF << (idx & 56));
    Mask { bit, right: bit ^ left ^ (0xFF << (idx & 56)), left, file: bit ^ FILE << (idx & 7) }
);
// Zobrist values
pub struct ZobristVals {
    pub pieces: [[[u64; 64]; 8]; 2],
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
pub static ZVALS: ZobristVals = {
    let mut seed = 180_620_142;
    seed = xor_shift(seed);
    let mut vals = ZobristVals { pieces: [[[0; 64]; 8]; 2], castle: [0; 4], en_passant: [0; 8], side: seed };
    let mut idx = 0;
    while idx < 2 {
        let mut piece = 2;
        while piece < 8 {
            let mut square = 0;
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
// Draw detection
pub const LSQ: u64 = 0x55AA_55AA_55AA_55AA;
pub const DSQ: u64 = 0xAA55_AA55_AA55_AA55;
