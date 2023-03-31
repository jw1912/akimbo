# akimbo
A compact UCI Chess(960) engine written in Rust.

### Compiling
If you have cargo installed, run `cargo build --release`.

### Aims
The main aim is to stay under 1500 lines of code, currently at 909 excluding blank lines and comments (995 including).

## Features

#### Evaluation
- Tapered PSTs

#### Selectivity
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
