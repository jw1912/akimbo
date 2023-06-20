// Structs
#[derive(Clone, Copy)]
struct Mask {
    bit: u64,
    diag: u64,
    anti: u64,
    file: u64,
}

pub struct ZobristVals {
    pub pcs: [[[u64; 64]; 8]; 2],
    pub cr: [u64; 16],
    pub enp: [u64; 8],
    pub c: [u64; 2],
}

#[derive(Clone, Copy, Default)]
pub struct S(pub i32, pub i32);

impl std::ops::AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl std::ops::SubAssign<S> for S {
    fn sub_assign(&mut self, rhs: S) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

// Macros
macro_rules! init {($i:ident, $size:expr, $($r:tt)+) => {{
    let mut $i = 0;
    let mut res = [{$($r)+}; $size];
    while $i < $size - 1 {
        $i += 1;
        res[$i] = {$($r)+};
    }
    res
}}}
macro_rules! c_enum {($t:ty, $name:ident, $($n:ident = $v:expr),*) => {
    pub struct $name;
    impl $name { $(pub const $n: $t = $v;)* }
}}

pub struct Attacks;
impl Attacks {
    pub const PAWN: [[u64; 64]; 2] = [
        init!(i, 64, (((1 << i) & !File::A) << 7) | (((1 << i) & !File::H) << 9)),
        init!(i, 64, (((1 << i) & !File::A) >> 9) | (((1 << i) & !File::H) >> 7)),
    ];

    pub const KNIGHT: [u64; 64] = init!(i, 64, {
        let n = 1 << i;
        let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
        let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
        (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
    });

    pub const KING: [u64; 64] = init!(i, 64, {
        let mut k = 1 << i;
        k |= (k << 8) | (k >> 8);
        k |= ((k & !File::A) >> 1) | ((k & !File::H) << 1);
        k ^ (1 << i)
    });

    pub fn bishop(idx: usize, occ: u64) -> u64 {
        let m = MASKS[idx];
        let rb = m.bit.swap_bytes();
        let (mut f1, mut f2) = (occ & m.diag, occ & m.anti);
        let (r1, r2) = (f1.swap_bytes().wrapping_sub(rb), f2.swap_bytes().wrapping_sub(rb));
        f1 = f1.wrapping_sub(m.bit);
        f2 = f2.wrapping_sub(m.bit);
        ((f1 ^ r1.swap_bytes()) & m.diag) | ((f2 ^ r2.swap_bytes()) & m.anti)
    }

    pub fn rook(idx: usize, occ: u64) -> u64 {
        let m = MASKS[idx];
        let mut f = occ & m.file;
        let i = idx & 7;
        let s = idx - i;
        let r = f.swap_bytes().wrapping_sub(m.bit.swap_bytes());
        f = f.wrapping_sub(m.bit);
        ((f ^ r.swap_bytes()) & m.file) | (RANKS[i][((occ >> (s + 1)) & 0x3F) as usize] << s)
    }
}

// All named collections of constants
c_enum!(u8, Bound, LOWER = 0, EXACT = 1, UPPER = 2);
c_enum!(i32, Score, MAX = 30000, MATE = Self::MAX - 256, DRAW = 0);
c_enum!(i32, MoveScore, HASH = 3000000, HISTORY_MAX = 65536, PROMO = 70000, KILLER = 69000);
c_enum!(usize, Side, WHITE = 0, BLACK = 1);
c_enum!(usize, Piece, EMPTY = 0, PAWN = 2, KNIGHT = 3, BISHOP = 4, ROOK = 5, QUEEN = 6, KING = 7);
c_enum!(u8, Flag, QUIET = 0, DBL = 1, KS = 2, QS = 3, CAP = 4, ENP = 5, PROMO = 8, QPR = 11, NPC = 12, QPC = 15);
c_enum!(u8, Rights, WQS = 8, WKS = 4, BQS = 2, BKS = 1, WHITE = Self::WQS | Self::WKS, BLACK = Self::BQS | Self::BKS);
c_enum!([u64; 2], Rank, PEN = [0xFF000000000000, 0xFF00], DBL = [0xFF000000, 0xFF00000000]);
c_enum!(u64, File, A = 0x101010101010101, H = Self::A << 7);

// Movegen
const EAST: [u64; 64] = init!(i, 64, (1 << i) ^ WEST[i] ^ (0xFF << (i & 56)));
const WEST: [u64; 64] = init!(i, 64, ((1 << i) - 1) & (0xFF << (i & 56)));
const DIAG: u64 = 0x8040201008040201;
const DIAGS: [u64; 15] = init!(i, 15, if i > 7 { DIAG >> (8 * (i - 7)) } else {DIAG << (8 * (7 - i)) });
static MASKS: [Mask; 64] = init!(i, 64,
    let (bit, rank, file) = (1 << i, i / 8, i & 7);
    Mask {
        bit,
        diag: bit ^ DIAGS[7 + file - rank],
        anti: bit ^ DIAGS[file + rank].swap_bytes(),
        file: bit ^ File::A << file,
    }
);
const RANKS: [[u64; 64]; 8] = init!(f, 8, init!(i, 64, {
    let occ = (i << 1) as u64;
      EAST[f] ^ EAST[( (EAST[f] & occ) | (1<<63)).trailing_zeros() as usize]
    | WEST[f] ^ WEST[(((WEST[f] & occ) | 1).leading_zeros() ^ 63) as usize]
}));
pub const CASTLE_MASK: [u8; 64] = init! {idx, 64, match idx {0 => 7, 4 => 3, 7 => 11, 56 => 13, 60 => 12, 63 => 14, _ => 15}};
pub const ROOK_MOVES: [[(u64, usize, usize); 2]; 2] = [[(9, 0, 3), (0x0900000000000000, 56, 59)], [(160, 7, 5), (0xA000000000000000, 63, 61)]];

// Zobrist values
const fn rand(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

pub static ZVALS: ZobristVals = {
    let mut seed = 180_620_142;
    seed = rand(seed);
    let c = [0, seed];
    let pcs = init!(side, 2, init!(pc, 8, init!(sq, 64, {
        if pc < 2 { 0 } else { seed = rand(seed); seed }
    })));
    let cf = init!(i, 4, {seed = rand(seed); seed});
    let cr = init!(i, 16, {
          ((i & 1 > 0) as u64 * cf[0]) ^ ((i & 2 > 0) as u64 * cf[1])
        ^ ((i & 4 > 0) as u64 * cf[2]) ^ ((i & 8 > 0) as u64 * cf[3])
    });
    let enp = init!(i, 8, {seed = rand(seed); seed});
    ZobristVals { pcs, cr, enp, c }
};

// Eval
pub const SIDE: [i32; 2] = [1, -1];
pub const PHASE_VALS: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];
pub static PST: [[[S; 64]; 8]; 2] = [
    init!(i, 8, init!(j, 64, RAW_PST[i][j ^ 56])),
    init!(i, 8, init!(j, 64, S(-RAW_PST[i][j].0, -RAW_PST[i][j].1))),
];
const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
    [
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        S(234, 248), S(247, 246), S(208, 224), S(212, 207), S(213, 211), S(217, 217), S(147, 256), S( 92, 273),
        S( 86, 189), S(105, 182), S(127, 168), S(142, 147), S(151, 140), S(144, 136), S(131, 169), S( 85, 170),
        S( 69, 132), S( 91, 121), S( 84, 108), S( 98,  95), S(103,  86), S( 91,  94), S(100, 109), S( 69, 108),
        S( 60, 112), S( 78, 107), S( 79,  91), S( 90,  84), S( 92,  85), S( 85,  87), S( 99,  95), S( 64,  91),
        S( 62, 100), S( 77, 102), S( 74,  89), S( 72,  97), S( 82,  92), S( 81,  89), S(122,  88), S( 80,  84),
        S( 54, 109), S( 75, 107), S( 62, 100), S( 51, 102), S( 64, 105), S( 98,  93), S(126,  92), S( 68,  85),
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
    ], [
        S(132, 240), S(217, 250), S(265, 276), S(294, 258), S(342, 256), S(222, 259), S(263, 228), S(175, 187),
        S(241, 265), S(266, 289), S(348, 267), S(353, 284), S(331, 277), S(373, 254), S(304, 257), S(284, 249),
        S(266, 270), S(346, 271), S(344, 298), S(359, 293), S(404, 275), S(414, 275), S(362, 272), S(348, 247),
        S(308, 270), S(309, 296), S(329, 308), S(342, 307), S(329, 308), S(358, 298), S(319, 298), S(333, 267),
        S(287, 266), S(303, 283), S(315, 303), S(307, 310), S(320, 304), S(310, 305), S(324, 286), S(302, 262),
        S(271, 253), S(296, 274), S(299, 284), S(299, 299), S(318, 291), S(308, 279), S(312, 267), S(276, 260),
        S(261, 244), S(249, 266), S(286, 265), S(298, 275), S(291, 281), S(307, 258), S(291, 259), S(287, 243),
        S(207, 239), S(272, 226), S(240, 267), S(263, 265), S(277, 259), S(296, 248), S(271, 240), S(246, 222),
    ], [
        S(260, 298), S(333, 282), S(255, 302), S(295, 295), S(270, 304), S(294, 296), S(307, 283), S(303, 279),
        S(291, 299), S(334, 307), S(323, 313), S(322, 300), S(356, 297), S(361, 291), S(323, 299), S(282, 285),
        S(306, 306), S(339, 304), S(354, 305), S(366, 305), S(361, 307), S(419, 297), S(368, 305), S(344, 299),
        S(307, 299), S(326, 317), S(356, 307), S(362, 318), S(352, 315), S(368, 308), S(329, 317), S(319, 303),
        S(330, 291), S(342, 302), S(333, 319), S(347, 319), S(351, 310), S(327, 315), S(330, 300), S(342, 286),
        S(324, 292), S(343, 298), S(333, 309), S(331, 315), S(327, 317), S(329, 306), S(339, 295), S(323, 290),
        S(342, 274), S(328, 286), S(333, 290), S(316, 308), S(322, 309), S(335, 289), S(339, 294), S(325, 265),
        S(288, 274), S(322, 284), S(304, 272), S(300, 293), S(315, 286), S(301, 287), S(303, 281), S(295, 278),
    ], [
        S(508, 504), S(519, 504), S(532, 504), S(539, 507), S(551, 501), S(513, 507), S(523, 504), S(529, 500),
        S(503, 509), S(509, 509), S(532, 509), S(540, 506), S(540, 500), S(547, 496), S(524, 502), S(534, 499),
        S(486, 507), S(506, 507), S(507, 508), S(527, 500), S(525, 496), S(539, 492), S(554, 488), S(518, 491),
        S(458, 505), S(479, 504), S(494, 508), S(505, 499), S(500, 498), S(510, 496), S(496, 496), S(480, 500),
        S(455, 494), S(459, 502), S(468, 505), S(480, 497), S(483, 491), S(463, 491), S(498, 483), S(463, 489),
        S(438, 488), S(459, 492), S(469, 486), S(465, 491), S(477, 480), S(479, 471), S(483, 479), S(461, 476),
        S(436, 487), S(460, 487), S(462, 493), S(469, 489), S(478, 480), S(477, 477), S(482, 478), S(407, 493),
        S(466, 476), S(474, 482), S(477, 494), S(486, 489), S(485, 481), S(474, 479), S(454, 490), S(458, 466),
    ], [
        S(882, 923), S(899, 953), S(919, 952), S(937, 940), S(958, 943), S(968, 932), S(968, 928), S(934, 944),
        S(878, 917), S(868, 952), S(905, 962), S(905, 973), S(905, 975), S(968, 944), S(921, 954), S(951, 922),
        S(895, 916), S(887, 941), S(898, 948), S(918, 963), S(940, 969), S(980, 955), S(963, 946), S(942, 935),
        S(890, 907), S(885, 949), S(892, 960), S(897, 965), S(917, 965), S(919, 958), S(915, 967), S(916, 944),
        S(893, 904), S(885, 937), S(891, 940), S(897, 965), S(902, 952), S(899, 956), S(911, 941), S(909, 932),
        S(893, 886), S(905, 885), S(895, 928), S(894, 918), S(895, 919), S(909, 912), S(913, 921), S(899, 913),
        S(867, 903), S(901, 891), S(910, 874), S(901, 894), S(902, 894), S(913, 878), S(884, 891), S(902, 884),
        S(909, 864), S(892, 873), S(886, 892), S(906, 864), S(889, 894), S(873, 872), S(852, 895), S(850, 869),
    ], [
        S(-12, -57), S( 78, -24), S(106, -29), S( 49,  -3), S(  8,  14), S( 14,  26), S( 45,  23), S( 15, -14),
        S( 77,  -9), S( 30,  34), S( 21,  32), S( 68,  23), S( 18,  31), S( 19,  49), S( 11,  56), S( -40, 28),
        S( 43,  10), S( 29,  34), S( 16,  38), S( 17,  32), S(  8,  37), S( 50,  48), S( 41,  57), S( -6,  32),
        S( 11,   1), S(  9,  30), S( -2,  39), S(-15,  41), S(-18,  39), S(  9,  42), S( 13,  38), S(-51,  21),
        S(-38,  -4), S( 22,   2), S(  1,  25), S(-36,  34), S(-26,  34), S(-18,  30), S(-27,  21), S(-53,   2),
        S( 17, -23), S( 13,  -2), S(-16,  14), S(-33,  24), S(-34,  26), S(-29,  19), S(  0,   5), S(-32,  -3),
        S( 26, -29), S( -6,  -4), S(-18,   8), S(-65,  17), S(-45,  17), S(-19,   7), S( 14, - 6), S( 26, -24),
        S(-44, -38), S( 19, -28), S(  2, -12), S(-72,  -5), S(  4, -33), S(-46,  -5), S( 33, -30), S( 25, -52),
    ],
];
