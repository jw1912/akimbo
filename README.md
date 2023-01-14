# akimbo
akimbo is a UCI compatible Chess (and Chess960) engine written in Rust.

### Aims
The main aim of akimbo is to stay under 1500 lines of code.
At last count it was at 1235 lines, excluding blank lines and comments.

### Compiling
If you have cargo installed, run `cargo build --release`.

### Parameter Tuning
Evaluation parameters are tuned using [akimbo_tuner](https://github.com/JacquesRW/akimbo_tuner).

## Features

#### Search
- Fail-Soft
- Principle Variation Search
- Quiescence Search
- Iterative Deepening
- Check Extensions

#### Move Ordering
1. Hash Move
2. Captures (MVV-LVA)
3. Promotions
4. Killer Moves
5. Quiets

#### Evaluation
- Material
- Knight and Bishop Mobility
- King and Pawn Tables

#### Pruning/Reductions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
