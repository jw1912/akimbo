#![allow(missing_docs)]

// engine details
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

// piece vals
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
    pub const ALL: u8 = 15;
    pub const WHITE_QS: u8 = 8;
    pub const WHITE_KS: u8 = 4;
    pub const BLACK_QS: u8 = 2;
    pub const BLACK_KS: u8 = 1;
    pub const SIDES: [u8; 2] = [
        Self::WHITE_KS | Self::WHITE_QS,
        Self::BLACK_KS | Self::BLACK_QS,
    ];
    pub const NONE: u8 = 0;
}

// movegen
pub const MSB: u64 = 0x80_00_00_00_00_00_00_00;
pub const LSB: u64 = 1;
pub const PENRANK: [u64; 2] = [0x00FF000000000000, 0x000000000000FF00];
pub const DBLRANK: [u64; 2] = [0x00000000FF000000, 0x000000FF00000000];
pub static NORTH: [u64; 64] = [72340172838076672, 144680345676153344, 289360691352306688, 578721382704613376, 1157442765409226752, 2314885530818453504, 4629771061636907008, 9259542123273814016, 72340172838076416, 144680345676152832, 289360691352305664, 578721382704611328, 1157442765409222656, 2314885530818445312, 4629771061636890624, 9259542123273781248, 72340172838010880, 144680345676021760, 289360691352043520, 578721382704087040, 1157442765408174080, 2314885530816348160, 4629771061632696320, 9259542123265392640, 72340172821233664, 144680345642467328, 289360691284934656, 578721382569869312, 1157442765139738624, 2314885530279477248, 4629771060558954496, 9259542121117908992, 72340168526266368, 144680337052532736, 289360674105065472, 578721348210130944, 1157442696420261888, 2314885392840523776, 4629770785681047552, 9259541571362095104, 72339069014638592, 144678138029277184, 289356276058554368, 578712552117108736, 1157425104234217472, 2314850208468434944, 4629700416936869888, 9259400833873739776, 72057594037927936, 144115188075855872, 288230376151711744, 576460752303423488, 1152921504606846976, 2305843009213693952, 4611686018427387904, 9223372036854775808, 0, 0, 0, 0, 0, 0, 0, 0];
pub static EAST: [u64; 64] = [254, 252, 248, 240, 224, 192, 128, 0, 65024, 64512, 63488, 61440, 57344, 49152, 32768, 0, 16646144, 16515072, 16252928, 15728640, 14680064, 12582912, 8388608, 0, 4261412864, 4227858432, 4160749568, 4026531840, 3758096384, 3221225472, 2147483648, 0, 1090921693184, 1082331758592, 1065151889408, 1030792151040, 962072674304, 824633720832, 549755813888, 0, 279275953455104, 277076930199552, 272678883688448, 263882790666240, 246290604621824, 211106232532992, 140737488355328, 0, 71494644084506624, 70931694131085312, 69805794224242688, 67553994410557440, 63050394783186944, 54043195528445952, 36028797018963968, 0, 18302628885633695744, 18158513697557839872, 17870283321406128128, 17293822569102704640, 16140901064495857664, 13835058055282163712, 9223372036854775808, 0];
pub static SOUTH: [u64; 64] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 4, 8, 16, 32, 64, 128, 257, 514, 1028, 2056, 4112, 8224, 16448, 32896, 65793, 131586, 263172, 526344, 1052688, 2105376, 4210752, 8421504, 16843009, 33686018, 67372036, 134744072, 269488144, 538976288, 1077952576, 2155905152, 4311810305, 8623620610, 17247241220, 34494482440, 68988964880, 137977929760, 275955859520, 551911719040, 1103823438081, 2207646876162, 4415293752324, 8830587504648, 17661175009296, 35322350018592, 70644700037184, 141289400074368, 282578800148737, 565157600297474, 1130315200594948, 2260630401189896, 4521260802379792, 9042521604759584, 18085043209519168, 36170086419038336];
pub static WEST: [u64; 64] = [0, 1, 3, 7, 15, 31, 63, 127, 0, 256, 768, 1792, 3840, 7936, 16128, 32512, 0, 65536, 196608, 458752, 983040, 2031616, 4128768, 8323072, 0, 16777216, 50331648, 117440512, 251658240, 520093696, 1056964608, 2130706432, 0, 4294967296, 12884901888, 30064771072, 64424509440, 133143986176, 270582939648, 545460846592, 0, 1099511627776, 3298534883328, 7696581394432, 16492674416640, 34084860461056, 69269232549888, 139637976727552, 0, 281474976710656, 844424930131968, 1970324836974592, 4222124650659840, 8725724278030336, 17732923532771328, 35747322042253312, 0, 72057594037927936, 216172782113783808, 504403158265495552, 1080863910568919040, 2233785415175766016, 4539628424389459968, 9151314442816847872];
pub static NE: [u64; 64] = [9241421688590303744, 36099303471055872, 141012904183808, 550831656960, 2151686144, 8404992, 32768, 0, 4620710844295151616, 9241421688590303232, 36099303471054848, 141012904181760, 550831652864, 2151677952, 8388608, 0, 2310355422147510272, 4620710844295020544, 9241421688590041088, 36099303470530560, 141012903133184, 550829555712, 2147483648, 0, 1155177711056977920, 2310355422113955840, 4620710844227911680, 9241421688455823360, 36099303202095104, 141012366262272, 549755813888, 0, 577588851233521664, 1155177702467043328, 2310355404934086656, 4620710809868173312, 9241421619736346624, 36099165763141632, 140737488355328, 0, 288793326105133056, 577586652210266112, 1155173304420532224, 2310346608841064448, 4620693217682128896, 9241386435364257792, 36028797018963968, 0, 144115188075855872, 288230376151711744, 576460752303423488, 1152921504606846976, 2305843009213693952, 4611686018427387904, 9223372036854775808, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub static SE: [u64; 64] = [0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 8, 16, 32, 64, 128, 0, 516, 1032, 2064, 4128, 8256, 16512, 32768, 0, 132104, 264208, 528416, 1056832, 2113664, 4227072, 8388608, 0, 33818640, 67637280, 135274560, 270549120, 541097984, 1082130432, 2147483648, 0, 8657571872, 17315143744, 34630287488, 69260574720, 138521083904, 277025390592, 549755813888, 0, 2216338399296, 4432676798592, 8865353596928, 17730707128320, 35461397479424, 70918499991552, 140737488355328, 0, 567382630219904, 1134765260439552, 2269530520813568, 4539061024849920, 9078117754732544, 18155135997837312, 36028797018963968, 0];
pub static SW: [u64; 64] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 4, 8, 16, 32, 64, 0, 256, 513, 1026, 2052, 4104, 8208, 16416, 0, 65536, 131328, 262657, 525314, 1050628, 2101256, 4202512, 0, 16777216, 33619968, 67240192, 134480385, 268960770, 537921540, 1075843080, 0, 4294967296, 8606711808, 17213489152, 34426978560, 68853957121, 137707914242, 275415828484, 0, 1099511627776, 2203318222848, 4406653222912, 8813306511360, 17626613022976, 35253226045953, 70506452091906, 0, 281474976710656, 564049465049088, 1128103225065472, 2256206466908160, 4512412933881856, 9024825867763968, 18049651735527937];
pub static NW: [u64; 64] = [0, 256, 66048, 16909312, 4328785920, 1108169199616, 283691315109888, 72624976668147712, 0, 65536, 16908288, 4328783872, 1108169195520, 283691315101696, 72624976668131328, 145249953336262656, 0, 16777216, 4328521728, 1108168671232, 283691314053120, 72624976666034176, 145249953332068352, 290499906664136704, 0, 4294967296, 1108101562368, 283691179835392, 72624976397598720, 145249952795197440, 290499905590394880, 580999811180789760, 0, 1099511627776, 283673999966208, 72624942037860352, 145249884075720704, 290499768151441408, 580999536302882816, 1161999072605765632, 0, 281474976710656, 72620543991349248, 145241087982698496, 290482175965396992, 580964351930793984, 1161928703861587968, 2323857407723175936, 0, 72057594037927936, 144115188075855872, 288230376151711744, 576460752303423488, 1152921504606846976, 2305843009213693952, 4611686018427387904, 0, 0, 0, 0, 0, 0, 0, 0];
pub static KNIGHT_ATTACKS: [u64; 64] = [132096, 329728, 659712, 1319424, 2638848, 5277696, 10489856, 4202496, 33816580, 84410376, 168886289, 337772578, 675545156, 1351090312, 2685403152, 1075839008, 8657044482, 21609056261, 43234889994, 86469779988, 172939559976, 345879119952, 687463207072, 275414786112, 2216203387392, 5531918402816, 11068131838464, 22136263676928, 44272527353856, 88545054707712, 175990581010432, 70506185244672, 567348067172352, 1416171111120896, 2833441750646784, 5666883501293568, 11333767002587136, 22667534005174272, 45053588738670592, 18049583422636032, 145241105196122112, 362539804446949376, 725361088165576704, 1450722176331153408, 2901444352662306816, 5802888705324613632, 11533718717099671552, 4620693356194824192, 288234782788157440, 576469569871282176, 1224997833292120064, 2449995666584240128, 4899991333168480256, 9799982666336960512, 1152939783987658752, 2305878468463689728, 1128098930098176, 2257297371824128, 4796069720358912, 9592139440717824, 19184278881435648, 38368557762871296, 4679521487814656, 9077567998918656];
pub static KING_ATTACKS: [u64; 64] = [770, 1797, 3594, 7188, 14376, 28752, 57504, 49216, 197123, 460039, 920078, 1840156, 3680312, 7360624, 14721248, 12599488, 50463488, 117769984, 235539968, 471079936, 942159872, 1884319744, 3768639488, 3225468928, 12918652928, 30149115904, 60298231808, 120596463616, 241192927232, 482385854464, 964771708928, 825720045568, 3307175149568, 7718173671424, 15436347342848, 30872694685696, 61745389371392, 123490778742784, 246981557485568, 211384331665408, 846636838289408, 1975852459884544, 3951704919769088, 7903409839538176, 15806819679076352, 31613639358152704, 63227278716305408, 54114388906344448, 216739030602088448, 505818229730443264, 1011636459460886528, 2023272918921773056, 4046545837843546112, 8093091675687092224, 16186183351374184448, 13853283560024178688, 144959613005987840, 362258295026614272, 724516590053228544, 1449033180106457088, 2898066360212914176, 5796132720425828352, 11592265440851656704, 4665729213955833856];
pub static PAWN_ATTACKS: [[u64; 64];2] = [
    [512, 1280, 2560, 5120, 10240, 20480, 40960, 16384, 131072, 327680, 655360, 1310720, 2621440, 5242880, 10485760, 4194304, 33554432, 83886080, 167772160, 335544320, 671088640, 1342177280, 2684354560, 1073741824, 8589934592, 21474836480, 42949672960, 85899345920, 171798691840, 343597383680, 687194767360, 274877906944, 2199023255552, 5497558138880, 10995116277760, 21990232555520, 43980465111040, 87960930222080, 175921860444160, 70368744177664, 562949953421312, 1407374883553280, 2814749767106560, 5629499534213120, 11258999068426240, 22517998136852480, 45035996273704960, 18014398509481984, 144115188075855872, 360287970189639680, 720575940379279360, 1441151880758558720, 2882303761517117440, 5764607523034234880, 11529215046068469760, 4611686018427387904, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 2, 5, 10, 20, 40, 80, 160, 64, 512, 1280, 2560, 5120, 10240, 20480, 40960, 16384, 131072, 327680, 655360, 1310720, 2621440, 5242880, 10485760, 4194304, 33554432, 83886080, 167772160, 335544320, 671088640, 1342177280, 2684354560, 1073741824, 8589934592, 21474836480, 42949672960, 85899345920, 171798691840, 343597383680, 687194767360, 274877906944, 2199023255552, 5497558138880, 10995116277760, 21990232555520, 43980465111040, 87960930222080, 175921860444160, 70368744177664, 562949953421312, 1407374883553280, 2814749767106560, 5629499534213120, 11258999068426240, 22517998136852480, 45035996273704960, 18014398509481984]
];

// castling
pub const A1: u64 = 1;
pub const B1: u64 = 2;
pub const C1: u64 = 4;
pub const D1: u64 = 8;
pub const F1: u64 = 32;
pub const G1: u64 = 64;
pub const H1: u64 = 128;
pub const A8: u64 = 0x0100000000000000;
pub const B8: u64 = 0x0200000000000000;
pub const C8: u64 = 0x0400000000000000;
pub const D8: u64 = 0x0800000000000000;
pub const F8: u64 = 0x2000000000000000;
pub const G8: u64 = 0x4000000000000000;
pub const H8: u64 = 0x8000000000000000;
pub static CASTLE_RIGHTS: [u8; 64] = castle_rights();
const fn castle_rights() -> [u8; 64] {
    let mut rights: [u8; 64] = [CastleRights::ALL; 64];
    rights[0] = 0b0111;
    rights[7] = 0b1011;
    rights[56] = 0b1101;
    rights[63] = 0b1110;
    rights[4] = 0b0011;
    rights[60] = 0b1100;
    rights
}
pub const CASTLE_MOVES: [[(u64, usize, usize);2];2] = [[(A1|D1, 0, 3), (F1|H1, 7, 5)], [(A8|D8, 56, 59), (F8|H8, 63, 61)]];

// search/eval
pub const MAX_PLY: i8 = i8::MAX;
pub const MAX: i16 = 30000;
pub const MATE_THRESHOLD: i16 = MAX - u8::MAX as i16;
pub const SIDE_FACTOR: [i16; 3] = [1, -1, 0];
pub const PHASE_VALS: [i16; 7] = [0, 1, 1, 2, 4, 0, 0];
pub const TPHASE: i32 = 24;

// move ordering
pub const HASH_MOVE: u16 = 30000;
pub const KILLER: u16 = 500;
pub const QUIET: u16 = 0;
pub const MVV_LVA: [[u16; 7]; 7] = [[1500, 1400, 1300, 1200, 1100, 1000, 0], [2500, 2400, 2300, 2200, 2100, 2000, 0], [3500, 3400, 3300, 3200, 3100, 3000, 0], [4500, 4400, 4300, 4200, 4100, 4000, 0], [5500, 5400, 5300, 5200, 5100, 5000,0], [0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0, 0]];

// eval values
pub const PASSERS_MG: i16 = -2;
pub const PASSERS_EG: i16 = 30;
pub static PST_MG: [[i16; 64];6] = [
  [82, 82, 82, 82, 82, 82, 82, 82, 180, 216, 143, 177, 150, 208, 116, 71, 76, 89, 108, 113, 147, 138, 107, 62, 68, 95, 88, 103, 105, 94, 99, 59, 55, 80, 77, 94, 99, 88, 92, 57, 56, 78, 78, 72, 85, 85, 115, 70, 47, 81, 62, 59, 67, 106, 120, 60, 82, 82, 82, 82, 82, 82, 82, 82],
  [170, 248, 303, 288, 398, 240, 322, 230, 264, 296, 409, 373, 360, 399, 344, 320, 290, 397, 374, 402, 421, 466, 410, 381, 328, 354, 356, 390, 374, 406, 355, 359, 324, 341, 353, 350, 365, 356, 358, 329, 314, 328, 349, 347, 356, 354, 362, 321, 308, 284, 325, 334, 336, 355, 323, 318, 232, 316, 279, 304, 320, 309, 318, 314],
  [336, 369, 283, 328, 340, 323, 372, 357, 339, 381, 347, 352, 395, 424, 383, 318, 349, 402, 408, 405, 400, 415, 402, 363, 361, 370, 384, 415, 402, 402, 372, 363, 359, 378, 378, 391, 399, 377, 375, 369, 365, 380, 380, 380, 379, 392, 383, 375, 369, 380, 381, 365, 372, 386, 398, 366, 332, 362, 351, 344, 352, 353, 326, 344],
  [509, 519, 509, 528, 540, 486, 508, 520, 504, 509, 535, 539, 557, 544, 503, 521, 472, 496, 503, 513, 494, 522, 538, 493, 453, 466, 484, 503, 501, 512, 469, 457, 441, 451, 465, 476, 486, 470, 483, 454, 432, 452, 461, 460, 480, 477, 472, 444, 433, 461, 457, 468, 476, 488, 471, 406, 458, 464, 478, 494, 493, 484, 440, 451],
  [997, 1025, 1054, 1037, 1084, 1069, 1068, 1070, 1001, 986, 1020, 1026, 1009, 1082, 1053, 1079, 1012, 1008, 1032, 1033, 1054, 1081, 1072, 1082, 998, 998, 1009, 1009, 1024, 1042, 1023, 1026, 1016, 999, 1016, 1015, 1023, 1021, 1028, 1022, 1011, 1027, 1014, 1023, 1020, 1027, 1039, 1030, 990, 1017, 1036, 1027, 1033, 1040, 1022, 1026, 1024, 1007, 1016, 1035, 1010, 1000, 994, 975],
  [-65, 23, 16, -15, -56, -34, 2, 13, 29, -1, -20, -7, -8, -4, -38, -29, -9, 24, 2, -16, -20, 6, 22, -22, -17, -20, -12, -27, -30, -25, -14, -36, -49, -1, -27, -39, -46, -44, -33, -51, -14, -14, -22, -46, -44, -30, -15, -27, 1, 7, -8, -64, -43, -16, 9, 8, -15, 36, 12, -54, 8, -28, 24, 14],
]; 
pub static PST_EG: [[i16; 64];6] = [
  [94, 94, 94, 94, 94, 94, 94, 94, 272, 267, 252, 228, 241, 226, 259, 281, 188, 194, 179, 161, 150, 147, 176, 178, 126, 118, 107, 99, 92, 98, 111, 111, 107, 103, 91, 87, 87, 86, 97, 93, 98, 101, 88, 95, 94, 89, 93, 86, 107, 102, 102, 104, 107, 94, 96, 87, 94, 94, 94, 94, 94, 94, 94, 94],
  [223, 243, 268, 253, 250, 254, 218, 182, 256, 273, 256, 279, 272, 256, 257, 229, 257, 261, 291, 290, 280, 272, 262, 240, 264, 284, 303, 303, 303, 292, 289, 263, 263, 275, 297, 306, 297, 298, 285, 263, 258, 278, 280, 296, 291, 278, 261, 259, 239, 261, 271, 276, 279, 261, 258, 237, 252, 230, 258, 266, 259, 263, 231, 217],
  [283, 276, 286, 289, 290, 288, 280, 273, 289, 293, 304, 285, 294, 284, 293, 283, 299, 289, 297, 296, 295, 303, 297, 301, 294, 306, 309, 306, 311, 307, 300, 299, 291, 300, 310, 316, 304, 307, 294, 288, 285, 294, 305, 307, 310, 300, 290, 282, 283, 279, 290, 296, 301, 288, 282, 270, 274, 288, 274, 292, 288, 281, 292, 280],
  [525, 522, 530, 527, 524, 524, 520, 517, 523, 525, 525, 523, 509, 515, 520, 515, 519, 519, 519, 517, 516, 509, 507, 509, 516, 515, 525, 513, 514, 513, 511, 514, 515, 517, 520, 516, 507, 506, 504, 501, 508, 512, 507, 511, 505, 500, 504, 496, 506, 506, 512, 514, 503, 503, 501, 509, 503, 514, 515, 511, 507, 499, 516, 492],
  [927, 958, 958, 963, 963, 955, 946, 956, 919, 956, 968, 977, 994, 961, 966, 936, 916, 942, 945, 985, 983, 971, 955, 945, 939, 958, 960, 981, 993, 976, 993, 972, 918, 964, 955, 983, 967, 970, 975, 959, 920, 909, 951, 942, 945, 953, 946, 941, 914, 913, 906, 920, 920, 913, 900, 904, 903, 908, 914, 893, 931, 904, 916, 895],
  [-74, -35, -18, -18, -11, 15, 4, -17, -12, 17, 14, 17, 17, 38, 23, 11, 10, 17, 23, 15, 20, 45, 44, 13, -8, 22, 24, 27, 26, 33, 26, 3, -18, -4, 21, 24, 27, 23, 9, -11, -19, -3, 11, 21, 23, 16, 7, -9, -27, -11, 4, 13, 14, 4, -5, -17, -53, -34, -21, -11, -28, -14, -24, -43],
]; 

// king eval stuff
pub const CMD: [i16; 64] = [6, 5, 4, 3, 3, 4, 5, 6, 5, 4, 3, 2, 2, 3, 4, 5, 4, 3, 2, 1, 1, 2, 3, 4, 3, 2, 1, 0, 0, 1, 2, 3, 3, 2, 1, 0, 0, 1, 2, 3, 4, 3, 2, 1, 1, 2, 3, 4, 5, 4, 3, 2, 2, 3, 4, 5, 6, 5, 4, 3, 3, 4, 5, 6];
pub static MD: [[i16; 64]; 64] = manhattan();
const fn manhattan() -> [[i16; 64]; 64] {
    let mut res: [[i16; 64]; 64] = [[0; 64]; 64];
    let mut i: i16 = 0;
    while i < 64 {
        let mut j: i16 = 0;
        while j < 64 {
            res[i as usize][j as usize] = 5 * CMD[i as usize] + 2 * (14 - ((i >> 3) - (j>> 3)).abs() + ((i & 7) - (j & 7)).abs());
            j += 1;
        }
        i += 1;
    }
    res
}

// fen strings
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
pub const LASKER: &str = "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1";
pub const _POSITIONS: [&str; 12] = [
    STARTPOS, LASKER, KIWIPETE,
    "rn5r/pp3kpp/2p1R3/5p2/3P4/2B2N2/PPP3PP/2K4n w - - 1 17",
    "4r1rk/pp4pp/2n5/8/6Q1/7R/1qPK1P1P/3R4 w - - 0 28",
    "2r1rbk1/1R3R1N/p3p1p1/3pP3/8/q7/P1Q3PP/7K b - - 0 25",
    "8/2krR3/1pp3bp/6p1/PPNp4/3P1PKP/8/8 w - - 0 1",
    "1Q6/8/8/8/2k2P2/1p6/1B4K1/8 w - - 3 63",
    "3r2k1/pp3ppp/4p3/8/QP6/P1P5/5KPP/7q w - - 0 27",
    "1q1r3k/3P1pp1/ppBR1n1p/4Q2P/P4P2/8/5PK1/8 w - - 0 1",
    "1n3r2/3k2pp/pp1P4/1p4b1/1q3B2/5Q2/PPP2PP1/R4RK1 w - - 0 1",
    "7K/8/k1P5/7p/8/8/8/8 w - - 0 1"
];

// uci <-> u16
pub const FILES: [char; 8] = ['a','b','c','d','e','f','g','h'];
pub const PIECES: [char; 12] = ['P','N','B','R','Q','K','p','n','b','r','q','k'];
pub const PROMOS: [&str; 4] = ["n","b","r","q"];
pub const PROMO_BIT: u16 = 0b1000_0000_0000_0000;
pub const TWELVE: u16 = 0b0000_1111_1111_1111;

// KBvKB draw detection
pub const SQ1: u64 = 0x55AA55AA55AA55AA;
pub const SQ2: u64 = 0xAA55AA55AA55AA55;

// non-edge files
pub const NOT_A: u64 = 0xfefefefefefefefe;
pub const NOT_H: u64 = 0x7f7f7f7f7f7f7f7f;