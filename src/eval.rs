use std::ops::{AddAssign, Sub};

use crate::{attacks::Attacks, bitloop, consts::{File, Piece, Side, RAILS, SPANS}, position::Position};

pub fn eval(pos: &Position) -> i32 {
    let score = eval_side(pos, Side::WHITE) - eval_side(pos, Side::BLACK);

    [1, -1][pos.stm()] * score.taper(pos.phase)
}

fn eval_side(pos: &Position, side: usize) -> S {
    let mut score = S(0, 0);

    let side_bb = pos.side(side);
    let ksq = (pos.piece(Piece::KING) & side_bb).trailing_zeros();

    let flip = [56, 0][side] ^ if ksq % 8 > 3 { 7 } else { 0 };

    let pawns_bb = pos.piece(Piece::PAWN);

    let opp_pawns = pos.side(side ^ 1) & pawns_bb;

    let pawn_threats = if side == Side::WHITE {
        Attacks::black_pawn_setwise(opp_pawns)
    } else {
        Attacks::white_pawn_setwise(opp_pawns)
    };

    let safe = !pawn_threats;

    let occ = pos.side(side) ^ pos.side(side ^ 1);

    if (side_bb & pos.piece(Piece::BISHOP)).count_ones() > 1 {
        score += BISHOP_PAIR;
    }

    for (piece, pst) in PST.iter().enumerate().take(Piece::KING + 1).skip(Piece::PAWN) {
        let mut bb = pos.piece(piece) & side_bb;

        bitloop!(|bb, sq| {
            let sq = usize::from(sq);
            let fsq = sq ^ flip;
            score += pst[fsq];

            match piece {
                Piece::PAWN => {
                    if RAILS[sq % 8] & pawns_bb & side_bb == 0 {
                        score += ISOLATED_PAWN_FILE[fsq % 8];
                    }

                    if SPANS[side][sq] & pawns_bb & pos.side(side ^ 1) == 0 {
                        score += PASSED_PAWN_PST[fsq];
                    }
                }
                Piece::KNIGHT => score += KNIGHT_MOBILITY[(Attacks::knight(sq) & safe).count_ones() as usize],
                Piece::BISHOP => score += BISHOP_MOBILITY[(Attacks::bishop(sq, occ) & safe).count_ones() as usize],
                Piece::ROOK => {
                    let file_bb = File::A << (sq % 8);
                    
                    if file_bb & pawns_bb & side_bb == 0 {
                        score += ROOK_SEMI_OPEN_FILE[fsq % 8];
                    }
    
                    if file_bb & pawns_bb == 0 {
                        score += ROOK_FULL_OPEN_FILE[fsq % 8];
                    }

                    score += ROOK_MOBILITY[(Attacks::rook(sq, occ) & safe).count_ones() as usize];
                }
                Piece::QUEEN => score += QUEEN_MOBILITY[(Attacks::queen(sq, occ) & safe).count_ones() as usize],
                _ => {}
            }
        });
    }

    score
}

#[derive(Clone, Copy, Default)]
pub struct S(i16, i16);

impl AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Sub<S> for S {
    type Output = S;

    fn sub(mut self, rhs: S) -> S {
        self.0 -= rhs.0;
        self.1 -= rhs.1;

        self
    }
}

impl S {
    fn taper(self, mut phase: i32) -> i32 {
        let mg = i32::from(self.0);
        let eg = i32::from(self.1);

        phase = phase.min(24);

        (mg * phase + (24 - phase) * eg) / 24
    }
}

static PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
[
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        S(212, 144), S(109, 165), S(191, 142), S(193, 133), S(181, 133), S(192, 137), S(168, 149), S(195, 152),
        S(172,  86), S(157,  86), S(167,  84), S(115, 111), S( 91, 107), S(101,  99), S( 87, 100), S( 71, 124),
        S(136,  82), S( 97,  89), S(119,  81), S(104,  88), S( 91,  91), S( 77,  95), S( 71,  93), S( 65, 109),
        S(114,  78), S( 92,  84), S(117,  80), S( 96,  84), S( 90,  89), S( 80,  88), S( 64,  83), S( 59,  98),
        S(128,  73), S(128,  73), S(113,  85), S( 95,  94), S( 77,  98), S( 65,  92), S( 67,  79), S( 54,  96),
        S(124,  72), S(151,  68), S(144,  84), S( 88, 108), S( 72,  98), S( 70,  96), S( 63,  83), S( 59, 102),
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
    ], [
        S(226, 272), S(303, 298), S(204, 338), S(328, 310), S(257, 324), S(205, 340), S(233, 319), S(203, 307),
        S(329, 299), S(296, 325), S(390, 289), S(372, 304), S(351, 315), S(350, 300), S(297, 322), S(291, 313),
        S(347, 299), S(363, 302), S(418, 303), S(376, 315), S(362, 317), S(353, 313), S(331, 303), S(297, 310),
        S(357, 310), S(324, 318), S(363, 325), S(348, 328), S(376, 321), S(338, 327), S(321, 311), S(320, 315),
        S(332, 311), S(340, 301), S(332, 319), S(334, 325), S(329, 325), S(323, 321), S(321, 307), S(314, 309),
        S(294, 293), S(322, 293), S(304, 296), S(323, 308), S(313, 312), S(296, 299), S(298, 295), S(292, 289),
        S(306, 312), S(312, 295), S(316, 281), S(307, 292), S(313, 287), S(296, 284), S(278, 303), S(283, 282),
        S(276, 280), S(306, 280), S(307, 282), S(296, 290), S(288, 289), S(267, 282), S(299, 270), S(264, 274),
    ], [
        S(293, 318), S(291, 323), S(185, 333), S(229, 327), S(228, 330), S(225, 324), S(228, 329), S(256, 327),
        S(320, 310), S(294, 316), S(340, 310), S(314, 313), S(305, 321), S(318, 318), S(319, 312), S(299, 318),
        S(345, 314), S(360, 314), S(366, 312), S(365, 315), S(348, 310), S(330, 320), S(342, 312), S(319, 317),
        S(322, 318), S(334, 319), S(353, 316), S(353, 318), S(372, 317), S(342, 310), S(334, 320), S(313, 318),
        S(348, 308), S(330, 313), S(324, 315), S(357, 314), S(353, 315), S(331, 322), S(323, 318), S(326, 308),
        S(343, 314), S(339, 307), S(337, 312), S(343, 316), S(337, 314), S(326, 313), S(337, 312), S(333, 307),
        S(348, 311), S(362, 295), S(350, 302), S(332, 311), S(323, 307), S(338, 300), S(327, 295), S(356, 302),
        S(323, 313), S(342, 303), S(327, 307), S(317, 309), S(309, 306), S(333, 291), S(336, 296), S(321, 298),
    ], [
        S(547, 529), S(528, 549), S(511, 558), S(519, 540), S(515, 532), S(509, 542), S(501, 549), S(514, 538),
        S(516, 543), S(514, 557), S(537, 554), S(517, 547), S(523, 540), S(512, 550), S(491, 558), S(489, 554),
        S(482, 552), S(535, 549), S(545, 547), S(547, 534), S(520, 534), S(519, 546), S(509, 547), S(491, 550),
        S(480, 541), S(487, 556), S(510, 552), S(523, 534), S(513, 532), S(492, 547), S(474, 553), S(469, 547),
        S(452, 526), S(467, 537), S(470, 541), S(485, 525), S(477, 528), S(465, 539), S(458, 540), S(455, 534),
        S(462, 507), S(482, 508), S(480, 517), S(484, 508), S(471, 511), S(458, 523), S(452, 526), S(450, 517),
        S(417, 517), S(472, 506), S(493, 503), S(492, 498), S(480, 499), S(475, 510), S(457, 513), S(452, 509),
        S(465, 495), S(459, 514), S(499, 505), S(504, 496), S(491, 501), S(486, 508), S(478, 503), S(478, 506),
    ], [
        S(1024, 967), S(1000, 978), S(998, 997), S(970, 1004), S(947, 1011), S(939, 1011), S(940, 997), S(933, 983),
        S(997, 969), S(897, 1040), S(956, 1028), S(907, 1065), S(905, 1052), S(901, 1039), S(881, 1033), S(911, 995),
        S(974, 1002), S(996, 1003), S(1002, 1012), S(967, 1023), S(919, 1038), S(933, 1006), S(921, 998), S(940, 954),
        S(966, 989), S(942, 1022), S(937, 1038), S(941, 1028), S(935, 1019), S(918, 1009), S(924, 991), S(929, 962),
        S(945, 978), S(945, 988), S(932, 994), S(930, 998), S(925, 1002), S(932, 976), S(921, 970), S(935, 942),
        S(951, 918), S(953, 935), S(952, 946), S(946, 946), S(936, 950), S(933, 953), S(941, 930), S(929, 935),
        S(942, 878), S(967, 874), S(966, 880), S(955, 909), S(953, 910), S(951, 910), S(941, 919), S(939, 919),
        S(919, 871), S(907, 882), S(939, 871), S(956, 881), S(964, 884), S(943, 903), S(942, 901), S(941, 910),
    ], [
        S( 84, -49), S( 77,   0), S( 68,  -7), S( 83, -17), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S( 55,  -0), S( 65,  30), S( 96,  19), S( 73,  13), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S( 10,  22), S( 59,  38), S( 68,  32), S( 78,  23), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(-25,  13), S(-16,  38), S( 14,  35), S(  7,  38), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(-75,   9), S(-67,  27), S(-59,  35), S(-72,  43), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(-72,   7), S(-45,  13), S(-71,  25), S(-74,  31), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(  1, -16), S(  5,  -7), S(-48,  10), S(-64,  13), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(  7, -56), S( 23, -37), S(-26, -23), S( -5, -34), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    ],
];

const ROOK_SEMI_OPEN_FILE: [S; 8] = [S( 47,  23), S( 65,  -5), S( 26,  -8), S( 16,   3), S( 24,   4), S( 15,  -0), S( 19,   2), S( 16,  15)];

const ROOK_FULL_OPEN_FILE: [S; 8] = [S( 61, -50), S( 38, -23), S( 34,  -9), S( 20,   0), S( 13,   3), S( 19,   5), S( 21,  -4), S( 21, -15)];

const ISOLATED_PAWN_FILE: [S; 8] = [S(-47,  -3), S(-12, -13), S(-23, -13), S(-33, -15), S(-33, -16), S(-25, -17), S(-18, -10), S(-15, -24)];

static PASSED_PAWN_PST: [S; 64] = [
    S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    S(112,  44), S(  9,  65), S( 91,  42), S( 93,  33), S( 81,  33), S( 92,  37), S( 68,  49), S( 95,  52),
    S( 27,  99), S( 28,  94), S( 51,  76), S( 48,  36), S( 71,  36), S( 73,  56), S(102,  61), S(111,  82),
    S( 16,  64), S( 41,  68), S( 13,  54), S( 12,  36), S( 32,  26), S( 37,  32), S( 52,  42), S( 53,  59),
    S( -8,  53), S(  8,  56), S(-10,  36), S( -6,  26), S(  0,  22), S( -4,  23), S( 25,  35), S( 29,  44),
    S(  4,  17), S(-14,  30), S( -5,  13), S( -1,  14), S(-14,  10), S(-10,  14), S( -6,  25), S( 12,  24),
    S(  8,  10), S(  6,  19), S( -4,  10), S( 18,  -1), S( -4,   4), S( -9,  18), S( -2,  27), S(  6,  21),
    S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
];

static KNIGHT_MOBILITY: [S; 9] = [
    S(-77, -57), S(-28, -52), S( -3, -17),
    S( 16,  10), S( 31,  20), S( 34,  34),
    S( 49,  34), S( 61,  36), S( 76,  28),
];

static BISHOP_MOBILITY: [S; 14] = [
    S(-70, -106), S(-36, -52), S( -1, -26), S(  9,   4), S( 24,  11), S( 38,  19), S( 46,  26),
    S( 52,  31), S( 56,  35), S( 60,  37), S( 65,  40), S( 76,  36), S( 92,  31), S(109,  33),
];

static ROOK_MOBILITY: [S; 15] = [
    S(-48, -37), S(-30, -16), S(-37,  10), S(-35,  39), S(-28,  47),
    S(-23,  53), S(-18,  60), S(-14,  66), S( -9,  68), S( -2,  71),
    S(  1,  74), S(  6,  75), S( 13,  77), S( 22,  72), S( 18,  76),
];

static QUEEN_MOBILITY: [S; 28] = [
    S(  0,   0), S(  0,   0), S(-11,  -7), S(  6, -26), S(  3,   3), S( 34,  16), S( 36,  53),
    S( 37,  85), S( 42,  98), S( 47, 115), S( 53, 126), S( 60, 127), S( 67, 128), S( 71, 137),
    S( 75, 137), S( 75, 146), S( 78, 151), S( 78, 156), S( 84, 154), S( 90, 154), S( 93, 157),
    S(101, 154), S(117, 145), S(131, 137), S(138, 128), S(123, 137), S( 87, 110), S( 66,  85),
];

const BISHOP_PAIR: S = S( 34,  58);
