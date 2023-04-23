# akimbo
A compact UCI Chess engine written in Rust.

### Compiling
If you have cargo installed, run `cargo build --release`, binary will be in `/target/release`.

### Aims
The main aim is to stay under 1000 SLOC (excluding blank lines and comments) and 1500 TLOC (including).

As a result writing idiomatic Rust is not an aim of this project.

## Stats
|                           Version                                |     Release Date     | SLOC | TLOC | CCRL Blitz | CCRL 40/15 |
| :---------------------------------------------------------------:|:--------------------:|:----:|:----:|:----------:|:----------:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1) |    8th January 2023  | 1167 | 1381 |    n/a     |    2471    |
| [0.2.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.2.0) |   10th   April 2023  |  866 | 1000 |    n/a     |     n/a    |
|                             dev                                  |          n/a         |  805 |  952 |    n/a     |     n/a    |

## Features

#### Evaluation
- Tapered PSTs

#### Selectivity
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
