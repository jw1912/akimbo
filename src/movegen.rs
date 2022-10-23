use super::{consts::*, position::{POS, is_square_attacked, MoveList}};

// macros
#[macro_export]
macro_rules! lsb {($x:expr) => {$x.trailing_zeros() as u16}}
macro_rules! msb {($x:expr) => {63 ^ $x.leading_zeros() as u16}}
#[macro_export]
macro_rules! pop {($x:expr) => {$x &= $x - 1}}

const ALL: u8 = 0;
const CAPTURES: u8 = 1;
const QUIETS: u8 = 2;
pub struct All;
pub struct Captures;
pub struct Quiets;
pub trait MoveType {const TYPE: u8;}
impl MoveType for All {const TYPE: u8 = ALL;}
impl MoveType for Captures {const TYPE: u8 = CAPTURES;}
impl MoveType for Quiets {const TYPE: u8 = QUIETS;}

fn encode_moves(move_list: &mut MoveList, mut attacks: u64, from: u16, flag: u16) {
    let f = from << 6;
    let mut aidx: u16;
    while attacks > 0 {
        aidx = lsb!(attacks);
        move_list.push(flag | f | aidx);
        pop!(attacks)
    }
}

// generate all moves of a given type in a position
pub fn gen_moves<U: MoveType>(move_list: &mut MoveList) {
    unsafe {
    let occupied = POS.sides[0] | POS.sides[1];
    let friendly = POS.sides[POS.side_to_move];
    if U::TYPE != CAPTURES && POS.state.castle_rights & CastleRights::SIDES[POS.side_to_move] > 0 {
        castles(move_list, occupied, friendly);
    }
    match POS.side_to_move {
        0 => pawn_moves_general::<{ WHITE }, U>(move_list, occupied),
        1 => pawn_moves_general::<{ BLACK }, U>(move_list, occupied),
        _ => panic!("Invalid side to move!"),
    }
    piece_moves_general::<{ KNIGHT }, U>(move_list, occupied, friendly);
    piece_moves_general::<{ BISHOP }, U>(move_list, occupied, friendly);
    piece_moves_general::<{ ROOK   }, U>(move_list, occupied, friendly);
    piece_moves_general::<{ QUEEN  }, U>(move_list, occupied, friendly);
    piece_moves_general::<{ KING   }, U>(move_list, occupied, friendly);
    }
}

unsafe fn pawn_moves_general<const SIDE: usize, U: MoveType>(move_list: &mut MoveList, occupied: u64) {
    let pawns = POS.pieces[PAWN] & POS.sides[SIDE];
    if U::TYPE != CAPTURES {
        pawn_pushes_general::<SIDE>(move_list, pawns, occupied);
    }
    if U::TYPE != QUIETS {
        let opps = POS.sides[SIDE ^ 1];
        pawn_captures_general::<SIDE>(move_list, pawns, opps);
        if POS.state.en_passant_sq > 0 { en_passants::<SIDE>(move_list, pawns, POS.state.en_passant_sq) }
    }
}

unsafe fn piece_moves_general<const PIECE: usize, U: MoveType>(
    move_list: &mut MoveList, occupied: u64, friendly: u64,
) { 
    let mut from: u16;
    let mut idx: usize;
    let mut attacks: u64;
    let mut attackers = POS.pieces[PIECE] & friendly;
    while attackers > 0 {
        from = lsb!(attackers);
        idx = from as usize;
        attacks = match PIECE {
            KNIGHT => KNIGHT_ATTACKS[idx],
            ROOK => rook_attacks(idx, occupied),
            BISHOP => bishop_attacks(idx, occupied),
            QUEEN => rook_attacks(idx, occupied) | bishop_attacks(idx, occupied),
            KING => KING_ATTACKS[idx],
            _ => panic!("Not a valid usize in fn piece_moves_general: {}", PIECE),
        };
        if U::TYPE != CAPTURES {
            encode_moves(move_list, attacks & !occupied, from, MoveFlags::QUIET);
        }
        if U::TYPE != QUIETS {
            encode_moves(move_list, attacks & POS.sides[POS.side_to_move ^ 1], from, MoveFlags::CAPTURE);
        }
        pop!(attackers)
    }
}

unsafe fn castles(move_list: &mut MoveList, occupied: u64, friendly: u64) {
    let king_idx = lsb!(POS.pieces[KING] & friendly) as usize;
    if is_square_attacked(king_idx, POS.side_to_move, occupied) {
        return
    }
    match POS.side_to_move {
        WHITE => {
            if POS.state.castle_rights & CastleRights::WHITE_QS > 0 && occupied & (B1 | C1 | D1) == 0
                && !is_square_attacked(3, WHITE, occupied) {
                move_list.push(MoveFlags::QS_CASTLE | 2 | 4 << 6)
            }
            if POS.state.castle_rights & CastleRights::WHITE_KS > 0 && occupied & (F1 | G1) == 0
                && !is_square_attacked(5, WHITE, occupied) {
                move_list.push(MoveFlags::KS_CASTLE | 6 | 4 << 6)
            }
        }
        BLACK => {
            if POS.state.castle_rights & CastleRights::BLACK_QS > 0 && occupied & (B8 | C8 | D8) == 0
                && !is_square_attacked(59, BLACK, occupied) {
                move_list.push(MoveFlags::QS_CASTLE | 58 | 60 << 6)
            }
            if POS.state.castle_rights & CastleRights::BLACK_KS > 0 && occupied & (F8 | G8) == 0
                && !is_square_attacked(61, BLACK, occupied) {
                move_list.push(MoveFlags::KS_CASTLE | 62 | 60 << 6)
            }
        }
        _ => panic!("Invalid side for castling!"),
    }
}

// PAWN move generation code
fn shift<const SIDE: usize, const AMOUNT: u8>(bb: u64) -> u64 {
    match SIDE {
        WHITE => bb >> AMOUNT,
        BLACK => bb << AMOUNT,
        _ => panic!("Invalid side in fn shift!"),
    }
}

fn idx_shift<const SIDE: usize, const AMOUNT: u16>(idx: u16) -> u16 {
    match SIDE {
        WHITE => idx + AMOUNT,
        BLACK => idx - AMOUNT,
        _ => panic!("Invalid side in fn shift!"),
    }
}

fn pawn_captures_general<const SIDE: usize>(move_list: &mut MoveList, mut attackers: u64, opponents: u64) {
    let mut from: u16;
    let mut attacks: u64;
    let mut promo_attackers = attackers & PENRANK[SIDE];
    attackers &= !PENRANK[SIDE];
    while attackers > 0 {
        from = lsb!(attackers);
        attacks = PAWN_ATTACKS[SIDE][from as usize] & opponents;
        encode_moves(move_list, attacks, from, MoveFlags::CAPTURE);
        pop!(attackers)
    }
    let mut cidx: u16;
    while promo_attackers > 0 {
        from = lsb!(promo_attackers);
        attacks = PAWN_ATTACKS[SIDE][from as usize] & opponents;
        while attacks > 0 {
            cidx = lsb!(attacks);
            let f = from << 6;
            move_list.push(MoveFlags::KNIGHT_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::BISHOP_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::ROOK_PROMO_CAPTURE | cidx | f);
            move_list.push(MoveFlags::QUEEN_PROMO_CAPTURE | cidx | f);
            pop!(attacks)
        }
        pop!(promo_attackers)
    }
}

/// finds all unpinned-pawn pushes
fn pawn_pushes_general<const SIDE: usize>(
    move_list: &mut MoveList,
    pawns: u64,
    occupied: u64,
) {
    let empty = !occupied;
    let mut pushable_pawns = shift::<SIDE, 8>(empty) & pawns;
    let mut dbl_pushable_pawns = shift::<SIDE, 8>(shift::<SIDE, 8>(empty & DBLRANK[SIDE]) & empty) & pawns;
    let mut promotable_pawns = pushable_pawns & PENRANK[SIDE];
    pushable_pawns &= !PENRANK[SIDE];
    let mut idx: u16;
    while pushable_pawns > 0 {
        idx = lsb!(pushable_pawns);
        pop!(pushable_pawns);
        move_list.push(idx_shift::<SIDE, 8>(idx) | idx << 6);
    }
    while promotable_pawns > 0 {
        idx = lsb!(promotable_pawns);
        pop!(promotable_pawns);
        let to = idx_shift::<SIDE, 8>(idx);
        let f = idx << 6;
        move_list.push(MoveFlags::KNIGHT_PROMO | to | f);
        move_list.push(MoveFlags::BISHOP_PROMO | to | f);
        move_list.push(MoveFlags::ROOK_PROMO | to | f);
        move_list.push(MoveFlags::QUEEN_PROMO | to | f);
    }
    while dbl_pushable_pawns > 0 {
        idx = lsb!(dbl_pushable_pawns);
        pop!(dbl_pushable_pawns);
        move_list.push(MoveFlags::DBL_PUSH | idx_shift::<SIDE, 16>(idx) | idx << 6);
    }
}

fn en_passants<const SIDE: usize>(move_list: &mut MoveList, pawns: u64, sq: u16) {
    let mut attackers = PAWN_ATTACKS[SIDE ^ 1][sq as usize] & pawns;
    while attackers > 0 {
        let cidx = lsb!(attackers);
        move_list.push( MoveFlags::EN_PASSANT | sq | cidx << 6 );
        pop!(attackers)
    }
}

// ROOK + BISHOP ATTACKS
pub fn rook_attacks(idx: usize, occ: u64) -> u64 {
    let mut norths = NORTH[idx];
    let mut sq = lsb!(norths & occ | MSB) as usize;
    norths ^= NORTH[sq];
    let mut easts = EAST[idx];
    sq = lsb!(easts & occ | MSB) as usize;
    easts ^= EAST[sq];
    let mut souths = SOUTH[idx];
    sq = msb!(souths & occ | LSB) as usize;
    souths ^= SOUTH[sq];
    let mut wests = WEST[idx];
    sq = msb!(wests & occ | LSB) as usize;
    wests ^= WEST[sq];
    norths | easts | souths | wests
}

pub fn bishop_attacks(idx: usize, occ: u64) -> u64 {
    let mut nes = NE[idx];
    let mut sq = lsb!(nes & occ | MSB) as usize;
    nes ^= NE[sq];
    let mut nws = NW[idx];
    sq = lsb!(nws & occ | MSB) as usize;
    nws ^= NW[sq];
    let mut ses = SE[idx];
    sq = msb!(ses & occ | LSB) as usize;
    ses ^= SE[sq];
    let mut sws = SW[idx];
    sq = msb!(sws & occ | LSB) as usize;
    sws ^= SW[sq];
    nes | nws | ses | sws
}