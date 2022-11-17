# akimbo

akimbo is a UCI compatible chess engine written in Rust.
It is the successor of [Kimbo](https://github.com/JacquesRW/Kimbo).

### Aims
The main aim of akimbo is to stay under 1500 lines of code.
At last count it was at 1264 lines, excluding blank lines and comments.

### Compiling
If you have cargo installed, run `cargo build --release`.

## Features

#### Move Generation
- Bitboards
- Pseudo-legal
- Hyperbola quintessence sliding attacks

#### Search
- Fail-hard
- Principle variation search
- Quiescence search
- Iterative deepening
- Check extensions

#### Move Ordering
1. Hash move
2. Captures (MVV-LVA)
3. Killer moves
4. Quiets

#### Evaluation
- Tapered piece-square tables

#### Pruning/Reductions
- Mate distance pruning
- Hash score pruning
- Late move reductions
- Reverse futility pruning
- Null move pruning
- Delta pruning
