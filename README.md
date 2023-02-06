# akimbo
A UCI Chess(960) engine written in Rust.

### Compiling
If you have cargo installed, run `cargo build --release`.

## Features

#### Evaluation
- Material
- Knight, Bishop and Rook Mobility
- Pawn Half-Table
- King Quarter-Table

#### Selectivity
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
