# akimbo
A compact UCI Chess engine written in Rust.

### Compiling
If you have cargo installed, run `cargo build --release`.

### Aims
The main aim is to stay under 1500 SLOC (excluding blank lines and comments) and 2000 TLOC (including).

## Stats
|                             Version                               |     Release Date     | SLOC | TLOC | CCRL Blitz | CCRL 40/15 |
| :----------------------------------------------------------------:| :-------------------:|:----:|:----:|:----------:|:----------:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1)  |   8th January 2022   | 1167 | 1381 |    n/a     |    2475    |
|                               dev                                 |          n/a         |  929 | 1023 |    n/a     |     n/a    |

## Features

#### Evaluation
- Tapered PSTs

#### Selectivity
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
