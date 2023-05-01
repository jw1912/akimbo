# akimbo

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/JacquesRW/akimbo?style=for-the-badge)](https://github.com/JacquesRW/akimbo/releases/latest)
![Commits since latest release](https://img.shields.io/github/commits-since/JacquesRW/akimbo/latest?style=for-the-badge)
![TLOC](https://img.shields.io/tokei/lines/github/JacquesRW/akimbo?style=for-the-badge)

A compact UCI Chess engine written in Rust.

### Compiling
If you have cargo installed, run `cargo build --release`, binary will be in `/target/release`.

To build optimised for your specific cpu, run `cargo rustc --release -- -C target-cpu=native` instead.

### Aims
The main aim is to stay under 1000 SLOC (excluding blank lines and comments) and 1500 TLOC (including).

As a result writing idiomatic Rust is not an aim of this project.

## Stats
|                           Version                                |     Release Date     | SLOC | TLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/cgi/compare_engines.cgi?family=Akimbo) |
| :---------------------------------------------------------------:|:--------------------:|:----:|:----:|:-------------:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1) |    8th January 2023  | 1167 | 1381 |    2469       |
| [0.2.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.2.0) |   10th   April 2023  |  866 | 1000 |    2524       |
|                             dev                                  |          n/a         |  820 |  974 |     n/a       |

## Features

#### Structure
- Bitboards
- Hyperbola Quintessence / Rank Lookup
- Copy-Make

#### Evaluation
- Tapered PSTs

#### Selectivity
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning

#### Move Ordering
1. Hash Move
2. Promotions
3. Captures (MVV-LVA)
4. Killer Moves
5. History Heuristic
