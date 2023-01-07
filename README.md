# akimbo

akimbo is a UCI compatible Chess (and Chess960) engine written in Rust.

### Aims
The main aim of akimbo is to stay under 1500 lines of code.
At last count it was at 1165 lines, excluding blank lines and comments.

### Compiling
If you have cargo installed, run `cargo build --release`.

### Parameter Tuning
Piece-square tables were tuned using [akimbo_tuner](https://github.com/JacquesRW/akimbo_tuner).

## Features

#### Move Generation
- Bitboards
- Pseudo-legal
- Hyperbola quintessence sliding attacks

#### Search
- Fail-soft negamax
- Principle variation search
- Quiescence search
- Iterative deepening
- Check extensions

#### Move Ordering
1. Hash move
2. Captures (MVV-LVA)
3. Promotions
4. Killer moves
5. Quiets

#### Evaluation
- Tapered piece-square tables

#### Pruning/Reductions
- Mate distance pruning
- Hash score pruning
- Late move reductions
- Reverse futility pruning
- Null move pruning
- Delta pruning
