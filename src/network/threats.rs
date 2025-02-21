use crate::{consts::Piece, network::{attacks, indices, offsets}};

pub fn map_piece_threat(
    piece: usize,
    src: usize,
    dest: usize,
    target: usize,
    enemy: bool,
) -> Option<usize> {
    match piece {
        Piece::PAWN => map_pawn_threat(src, dest, target, enemy),
        Piece::KNIGHT => map_knight_threat(src, dest, target),
        Piece::BISHOP => map_bishop_threat(src, dest, target),
        Piece::ROOK => map_rook_threat(src, dest, target),
        Piece::QUEEN => map_queen_threat(src, dest, target),
        Piece::KING => map_king_threat(src, dest, target),
        _ => unreachable!(),
    }
}

fn below(src: usize, dest: usize, table: &[u64; 64]) -> usize {
    (table[src] & ((1 << dest) - 1)).count_ones() as usize
}

const fn offset_mapping<const N: usize>(a: [usize; N]) -> [usize; 12] {
    let mut res = [usize::MAX; 12];

    let mut i = 0;
    while i < N {
        res[a[i] - 2] = i;
        res[a[i] + 4] = i;
        i += 1;
    }

    res
}

fn target_is(target: usize, piece: usize) -> bool {
    target % 6 == piece - 2
}

fn map_pawn_threat(src: usize, dest: usize, target: usize, enemy: bool) -> Option<usize> {
    const MAP: [usize; 12] = offset_mapping([Piece::PAWN, Piece::KNIGHT, Piece::ROOK]);
    if MAP[target] == usize::MAX || (enemy && dest > src && target_is(target, Piece::PAWN)) {
        None
    } else {
        let diff = if dest > src { dest - src } else { src - dest };
        let attack = if diff == 7 { 0 } else { 1 } + 2 * (src % 8) - 1;
        let threat = offsets::PAWN + MAP[target] * indices::PAWN + (src / 8) * 14 + attack;

        assert!(threat < offsets::KNIGHT, "{threat}");

        Some(threat)
    }
}

fn map_knight_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    if dest > src && target_is(target, Piece::KNIGHT) {
        None
    } else {
        let idx = indices::KNIGHT[src] + below(src, dest, &attacks::KNIGHT);
        let threat = offsets::KNIGHT + target * indices::KNIGHT[64] + idx;

        assert!(threat >= offsets::KNIGHT, "{threat}");
        assert!(threat < offsets::BISHOP, "{threat}");

        Some(threat)
    }
}

fn map_bishop_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    const MAP: [usize; 12] = offset_mapping([
        Piece::PAWN,
        Piece::KNIGHT,
        Piece::BISHOP,
        Piece::ROOK,
        Piece::KING,
    ]);
    if MAP[target] == usize::MAX || dest > src && target_is(target, Piece::BISHOP) {
        None
    } else {
        let idx = indices::BISHOP[src] + below(src, dest, &attacks::BISHOP);
        let threat = offsets::BISHOP + MAP[target] * indices::BISHOP[64] + idx;

        assert!(threat >= offsets::BISHOP, "{threat}");
        assert!(threat < offsets::ROOK, "{threat}");

        Some(threat)
    }
}

fn map_rook_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    const MAP: [usize; 12] = offset_mapping([
        Piece::PAWN,
        Piece::KNIGHT,
        Piece::BISHOP,
        Piece::ROOK,
        Piece::KING,
    ]);
    if MAP[target] == usize::MAX || dest > src && target_is(target, Piece::ROOK) {
        None
    } else {
        let idx = indices::ROOK[src] + below(src, dest, &attacks::ROOK);
        let threat = offsets::ROOK + MAP[target] * indices::ROOK[64] + idx;

        assert!(threat >= offsets::ROOK, "{threat}");
        assert!(threat < offsets::QUEEN, "{threat}");

        Some(threat)
    }
}

fn map_queen_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    if dest > src && target_is(target, Piece::QUEEN) {
        None
    } else {
        let idx = indices::QUEEN[src] + below(src, dest, &attacks::QUEEN);
        let threat = offsets::QUEEN + target * indices::QUEEN[64] + idx;

        assert!(threat >= offsets::QUEEN, "{threat}");
        assert!(threat < offsets::KING, "{threat}");

        Some(threat)
    }
}

fn map_king_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    const MAP: [usize; 12] =
        offset_mapping([Piece::PAWN, Piece::KNIGHT, Piece::BISHOP, Piece::ROOK]);
    if MAP[target] == usize::MAX {
        None
    } else {
        let idx = indices::KING[src] + below(src, dest, &attacks::KING);
        let threat = offsets::KING + MAP[target] * indices::KING[64] + idx;

        assert!(threat >= offsets::KING, "{threat}");
        assert!(threat < offsets::END, "{threat}");

        Some(threat)
    }
}
