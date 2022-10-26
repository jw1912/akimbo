# akimbo

akimbo is a UCI compatible chess engine written in Rust. 
It is the successor of [Kimbo](https://github.com/JacquesRW/Kimbo), and its aim is be generally 
more efficient, and reach higher ELOs with fewer features.

### Compiling
If you have cargo installed, run `cargo build --release`.

## Features

#### Move Generation
- Bitboards
- Pseudo-legal
- Classical sliding attacks

#### Search
- Principle variation search
- Quiescence search
- Iterative deepening
- Check extensions

#### Move Ordering
1. Hash move
2. Captures, sorted by MVV-LVA
3. Killer moves
4. Quiets

#### Evalutaion
- Tapered from midgame to endgame
- Piece-square tables
- Passed pawn bonus
- Mop-up evaluation (for ladder mates, etc)

#### Pruning/Reductions
- Mate distance pruning
- Hash score pruning
- Late move reductions
- Reverse futility pruning
- Null move pruning
- Delta pruning
