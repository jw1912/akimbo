// Structs
#[derive(Clone, Copy)]
struct Mask {
    bit: u64,
    diag: u64,
    anti: u64,
    swap: u64,
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

impl std::ops::Mul<S> for i32 {
    type Output = S;
    fn mul(self, rhs: S) -> Self::Output {
        S(self * rhs.0, self * rhs.1)
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
        let (mut f1, mut f2) = (occ & m.diag, occ & m.anti);
        let (r1, r2) = (f1.swap_bytes().wrapping_sub(m.swap), f2.swap_bytes().wrapping_sub(m.swap));
        f1 = f1.wrapping_sub(m.bit);
        f2 = f2.wrapping_sub(m.bit);
        ((f1 ^ r1.swap_bytes()) & m.diag) | ((f2 ^ r2.swap_bytes()) & m.anti)
    }

    pub fn rook(sq: usize, occ: u64) -> u64 {
        FILE[sq][((((occ >> (sq & 7)) & File::A).wrapping_mul(DIAG) >> 57) & 0x3F) as usize]
        | RANK[sq][((occ >> RANK_SHIFT[sq]) & 0x3F) as usize]
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
        swap: bit.swap_bytes(),
    }
);
const RANK_SHIFT: [usize; 64] = init! {sq, 64, sq - (sq & 7) + 1};
const RANK: [[u64; 64]; 64] = init!(sq, 64, init!(i, 64, {
    let (f, occ) = (sq & 7, (i << 1) as u64);
    (EAST[f] ^ EAST[( (EAST[f] & occ) | (1<<63)).trailing_zeros() as usize]
    | WEST[f] ^ WEST[(((WEST[f] & occ) | 1).leading_zeros() ^ 63) as usize]) << (sq - f)
}));
const FILE: [[u64; 64]; 64] = init! {sq, 64, init! {occ, 64, (RANK[7 - sq / 8][occ].wrapping_mul(DIAG) & File::H) >> (7 - (sq & 7))}};

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
pub static PST: [[[S; 64]; 8]; 2] = [
        init!(i, 8, init!(j, 64, Eval::RAW_PST[i][j ^ 56])),
        init!(i, 8, init!(j, 64, S(-Eval::RAW_PST[i][j].0, -Eval::RAW_PST[i][j].1))),
];

pub struct Eval;
impl Eval {
    pub const SIDE: [i32; 2] = [1, -1];
    pub const PHASE: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];

    pub const PASSER: [S; 7] = [S(0, 0), S(-9, -4), S(-17, -1), S(-15, 25), S(9, 46), S(16, 88), S(30, 73)];
    pub const OPEN: [S; 8] = [S(37, 3), S(29, 4), S(26, 11), S(22, 25), S(30, 26), S(49, 1), S(54, 2), S(116, -11)];
    const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
        [
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
            S(134, 189), S(149, 174), S(128, 179), S(165, 127), S(139, 127), S(130, 137), S( 50, 179), S( 38, 193),
            S( 80, 141), S( 92, 147), S(117, 104), S(120,  74), S(129,  74), S(166,  81), S(136, 120), S(100, 124),
            S( 60, 123), S( 84, 115), S( 84,  94), S( 89,  71), S(112,  68), S(102,  82), S(107, 104), S( 86,  98),
            S( 49, 107), S( 75, 112), S( 72,  90), S( 89,  82), S( 86,  82), S( 80,  91), S( 92, 100), S( 70,  92),
            S( 45, 110), S( 71, 108), S( 67,  91), S( 63, 117), S( 77, 111), S( 72,  96), S(106, 100), S( 76,  90),
            S( 45, 109), S( 70, 107), S( 63,  94), S( 46, 115), S( 67, 125), S( 87, 102), S(116,  96), S( 66,  94),
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        ], [
            S(161, 211), S(216, 273), S(254, 296), S(282, 289), S(319, 290), S(257, 269), S(246, 271), S(203, 207),
            S(292, 277), S(310, 293), S(342, 295), S(358, 296), S(345, 286), S(402, 274), S(309, 286), S(321, 261),
            S(309, 287), S(348, 297), S(363, 314), S(378, 312), S(414, 298), S(419, 292), S(370, 289), S(339, 273),
            S(307, 299), S(323, 315), S(347, 330), S(372, 321), S(351, 323), S(377, 320), S(333, 308), S(343, 287),
            S(291, 308), S(309, 313), S(325, 335), S(324, 331), S(334, 340), S(331, 327), S(333, 303), S(303, 300),
            S(271, 292), S(295, 311), S(313, 289), S(316, 325), S(328, 320), S(315, 305), S(320, 293), S(289, 292),
            S(258, 282), S(270, 299), S(290, 303), S(300, 322), S(303, 301), S(306, 300), S(291, 290), S(294, 265),
            S(218, 266), S(267, 265), S(257, 286), S(272, 285), S(277, 296), S(291, 280), S(265, 293), S(248, 261),
        ], [
            S(304, 313), S(291, 323), S(302, 315), S(261, 331), S(273, 323), S(286, 314), S(320, 307), S(278, 311),
            S(332, 297), S(355, 311), S(351, 318), S(334, 317), S(363, 309), S(361, 305), S(350, 315), S(339, 288),
            S(344, 319), S(368, 316), S(368, 323), S(392, 311), S(382, 315), S(407, 319), S(386, 311), S(371, 309),
            S(332, 322), S(349, 333), S(371, 324), S(384, 334), S(379, 332), S(376, 326), S(349, 317), S(331, 324),
            S(328, 311), S(341, 330), S(349, 326), S(368, 332), S(365, 331), S(350, 322), S(339, 334), S(335, 299),
            S(335, 326), S(349, 305), S(345, 335), S(351, 307), S(351, 316), S(343, 336), S(349, 308), S(350, 308),
            S(344, 284), S(337, 344), S(358, 279), S(329, 334), S(334, 355), S(355, 300), S(356, 340), S(349, 273),
            S(316, 298), S(344, 285), S(317, 314), S(314, 313), S(318, 309), S(312, 327), S(343, 290), S(331, 290),
        ], [
            S(447, 557), S(449, 561), S(455, 563), S(465, 544), S(472, 539), S(484, 549), S(475, 549), S(446, 553),
            S(433, 557), S(439, 565), S(462, 561), S(489, 535), S(468, 537), S(483, 546), S(466, 546), S(447, 546),
            S(418, 554), S(450, 552), S(448, 548), S(461, 528), S(482, 517), S(489, 530), S(525, 522), S(466, 527),
            S(412, 555), S(432, 550), S(430, 552), S(445, 533), S(447, 521), S(452, 532), S(461, 531), S(440, 529),
            S(402, 547), S(408, 551), S(409, 549), S(429, 533), S(424, 531), S(413, 545), S(442, 531), S(420, 528),
            S(396, 543), S(406, 539), S(410, 536), S(418, 527), S(422, 522), S(422, 529), S(460, 504), S(432, 506),
            S(396, 536), S(405, 540), S(417, 537), S(421, 523), S(422, 520), S(427, 529), S(443, 519), S(405, 522),
            S(415, 539), S(418, 541), S(422, 545), S(434, 520), S(437, 514), S(426, 542), S(438, 531), S(412, 532),
        ], [
            S(912,1000), S(919,1010), S(948,1026), S(980,1015), S(985,1013), S(987,1003), S(999, 970), S(954, 990),
            S(953, 970), S(933,1001), S(938,1036), S(940,1042), S(953,1048), S(983,1016), S(959,1007), S(1001, 981),
            S(956, 974), S(954, 981), S(955,1025), S(976,1011), S(991,1023), S(1023,1013), S(1021, 982), S(1024, 953),
            S(937, 985), S(943,1009), S(946,1021), S(950,1037), S(958,1039), S(970,1029), S(972,1002), S(975, 985),
            S(925,1063), S(942,1003), S(936,1025), S(952,1019), S(951,1022), S(957, 994), S(967, 982), S(965, 975),
            S(935, 982), S(934,1048), S(942,1006), S(954, 948), S(961, 948), S(964, 949), S(981, 917), S(975, 903),
            S(931, 984), S(933,1005), S(941,1032), S(962, 914), S(954, 952), S(972, 909), S(969, 915), S(976, 883),
            S(931, 981), S(919, 997), S(925, 997), S(931,1069), S(939, 964), S(928, 960), S(950, 926), S(940, 935),
        ], [
            S(-22, -66), S(-24, -24), S( -6, -12), S(-47,  14), S(-28,   1), S( -4,   9), S( 19,  13), S( 13, -63),
            S(-54,   0), S(-40,  33), S(-67,  43), S( -3,  33), S(-21,  44), S(-17,  55), S( 16,  47), S(  3,  19),
            S(-78,  14), S( -3,  37), S(-54,  54), S(-68,  63), S(-34,  62), S( 21,  57), S( 10,  55), S(-19,  26),
            S(-71,  11), S(-75,  42), S(-90,  60), S(-124,  69), S(-117,  70), S(-87,  66), S(-85,  58), S(-104,  32),
            S(-75,   1), S(-79,  32), S(-112,  56), S(-143,  73), S(-139,  71), S(-105,  59), S(-103,  45), S(-131,  30),
            S(-32,  -6), S(-15,  16), S(-77,  37), S(-90,  51), S(-81,  52), S(-79,  41), S(-34,  22), S(-52,  11),
            S( 62, -28), S( 16,   1), S(  1,  11), S(-36,  21), S(-39,  24), S(-21,  18), S( 27,  -1), S( 38, -19),
            S( 54, -66), S( 79, -51), S( 47, -22), S(-57,  -4), S(  7, -15), S(-29,  -8), S( 52, -34), S( 55, -68),
        ],
    ];
}
