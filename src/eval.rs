use std::ops::{AddAssign, SubAssign};

use crate::{init, position::Position};

pub fn eval(pos: &Position) -> i32 {
    let score = pos.pst();

    [1, -1][pos.stm()] * score.taper(pos.phase)
}

#[derive(Clone, Copy, Default)]
pub struct S(i16, i16);

impl AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl SubAssign<S> for S {
    fn sub_assign(&mut self, rhs: S) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
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

pub static PST: [[[S; 64]; 8]; 2] = [
    init!(|i, 8| init!(|j, 64| RAW_PST[i][j ^ 56])),
    init!(|i, 8| init!(|j, 64| S(-RAW_PST[i][j].0, -RAW_PST[i][j].1))),
];
const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
    [
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        S(173, 284), S(225, 270), S(155, 257), S(203, 226), S(184, 238), S(234, 221), S(115, 271), S( 49, 303),
        S( 66, 200), S( 84, 204), S(108, 185), S(115, 165), S(157, 147), S(150, 144), S(103, 182), S( 57, 188),
        S( 58, 135), S( 93, 122), S( 89, 110), S(105,  99), S(107,  91), S( 96,  98), S(103, 111), S( 56, 116),
        S( 45, 116), S( 78, 107), S( 77,  93), S( 96,  87), S(101,  86), S( 91,  85), S( 96,  96), S( 54,  96),
        S( 47, 106), S( 76, 105), S( 78,  89), S( 72,  98), S( 87,  95), S( 89,  89), S(119,  90), S( 68,  88),
        S( 37, 117), S( 80, 106), S( 61, 107), S( 58, 107), S( 67, 108), S(109,  93), S(123,  94), S( 58,  90),
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
    ], [
        S(153, 234), S(244, 245), S(302, 271), S(305, 246), S(436, 233), S(249, 252), S(329, 213), S(230, 185),
        S(273, 254), S(304, 272), S(426, 246), S(383, 273), S(377, 265), S(420, 244), S(353, 252), S(338, 224),
        S(305, 251), S(417, 250), S(391, 284), S(421, 280), S(451, 262), S(487, 257), S(432, 249), S(402, 228),
        S(342, 259), S(372, 278), S(373, 298), S(409, 294), S(393, 295), S(429, 281), S(373, 280), S(376, 255),
        S(340, 256), S(359, 268), S(368, 292), S(366, 301), S(382, 290), S(374, 292), S(374, 279), S(346, 254),
        S(331, 250), S(344, 272), S(367, 271), S(365, 288), S(374, 284), S(373, 268), S(379, 252), S(339, 250),
        S(322, 235), S(296, 258), S(341, 265), S(351, 268), S(352, 271), S(373, 252), S(337, 252), S(336, 228),
        S(228, 256), S(334, 217), S(291, 256), S(316, 261), S(337, 252), S(324, 256), S(338, 220), S(330, 210),
    ], [
        S(349, 277), S(380, 270), S(267, 292), S(317, 289), S(333, 291), S(336, 284), S(368, 280), S(372, 267),
        S(356, 283), S(396, 286), S(362, 298), S(353, 284), S(416, 284), S(438, 276), S(400, 285), S(334, 280),
        S(366, 292), S(418, 280), S(425, 288), S(423, 287), S(416, 287), S(441, 291), S(415, 291), S(382, 294),
        S(377, 288), S(389, 299), S(400, 302), S(432, 298), S(422, 303), S(422, 298), S(390, 292), S(379, 293),
        S(375, 285), S(397, 291), S(395, 303), S(406, 309), S(416, 297), S(395, 299), S(392, 288), S(385, 282),
        S(382, 279), S(398, 287), S(396, 299), S(398, 300), S(395, 304), S(411, 291), S(399, 284), S(391, 276),
        S(387, 276), S(399, 271), S(398, 284), S(382, 290), S(391, 294), S(402, 281), S(416, 272), S(386, 261),
        S(343, 271), S(378, 283), S(369, 264), S(357, 287), S(365, 283), S(371, 272), S(339, 288), S(355, 278),
    ], [
        S(527, 526), S(547, 518), S(520, 532), S(570, 518), S(566, 519), S(487, 532), S(507, 525), S(523, 520),
        S(521, 523), S(522, 526), S(564, 518), S(567, 518), S(588, 499), S(579, 506), S(510, 524), S(534, 515),
        S(475, 527), S(504, 524), S(514, 521), S(518, 522), S(497, 522), S(543, 506), S(560, 503), S(500, 513),
        S(455, 526), S(472, 523), S(489, 532), S(512, 517), S(504, 520), S(521, 515), S(481, 516), S(461, 524),
        S(440, 527), S(454, 527), S(472, 528), S(481, 523), S(487, 515), S(474, 514), S(495, 507), S(458, 510),
        S(434, 520), S(458, 521), S(468, 515), S(463, 520), S(483, 511), S(480, 507), S(480, 509), S(450, 506),
        S(434, 519), S(468, 513), S(462, 521), S(471, 523), S(481, 511), S(491, 510), S(478, 507), S(411, 524),
        S(462, 514), S(468, 523), S(482, 524), S(492, 521), S(494, 516), S(480, 513), S(451, 523), S(461, 496),
    ], [
        S(950, 967), S(970, 1004), S(994, 1003), S(993, 1003), S(1095, 956), S(1098, 941), S(1037, 964), S(1025, 989),
        S(961, 954), S(940, 998), S(976, 1009), S(977, 1023), S(950, 1050), S(1051, 983), S(1008, 1000), S(1037, 962),
        S(972, 950), S(966, 979), S(990, 978), S(980, 1033), S(1006, 1026), S(1052, 990), S(1031, 987), S(1043, 971),
        S(948, 987), S(953, 1002), S(965, 1000), S(964, 1024), S(975, 1040), S(993, 1019), S(975, 1043), S(978, 1017),
        S(974, 950), S(949, 1013), S(971, 997), S(969, 1028), S(975, 1012), S(978, 1010), S(981, 1016), S(979, 997),
        S(962, 968), S(987, 936), S(969, 991), S(979, 979), S(975, 987), S(983, 994), S(994, 984), S(983, 983),
        S(946, 954), S(972, 952), S(995, 937), S(983, 956), S(990, 957), S(997, 947), S(982, 931), S(984, 938),
        S(979, 939), S(966, 940), S(973, 949), S(994, 916), S(964, 969), S(950, 947), S(947, 955), S(929, 932),
    ], [
        S(-41, -67), S(155, -57), S(140, -40), S( 85, -33), S(-106,  10), S(-65,  29), S( 56,  -1), S( 46, -15),
        S(174, -42), S( 62,   8), S( 45,   6), S(113,  -2), S( 50,   9), S( 36,  34), S(-31,  31), S(-123,  38),
        S( 66,  -1), S( 69,  12), S( 87,  11), S( 28,  13), S( 45,  13), S(113,  29), S(116,  29), S( -3,  15),
        S(  2,  -9), S(-10,  25), S( 17,  24), S(-34,  35), S(-28,  32), S(-33,  42), S( -4,  31), S(-72,  17),
        S(-68,  -9), S( 20,  -5), S(-53,  32), S(-101,  44), S(-104,  47), S(-68,  37), S(-59,  23), S(-79,   3),
        S( 13, -22), S( -6,  -0), S(-36,  20), S(-72,  35), S(-69,  38), S(-60,  31), S(-14,  13), S(-33,  -1),
        S( 17, -31), S( 18, -12), S(-14,  11), S(-76,  25), S(-57,  26), S(-28,  14), S( 18,  -5), S( 26, -22),
        S( -8, -56), S( 50, -44), S( 19, -24), S(-70,  -0), S(  7, -29), S(-36,  -5), S( 38, -32), S( 36, -57),
    ],
];