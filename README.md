<div align="center">

# akimbo

![License](https://img.shields.io/github/license/JacquesRW/akimbo?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/JacquesRW/akimbo?style=for-the-badge)](https://github.com/JacquesRW/akimbo/releases/latest)

</div>

A compact UCI Chess engine written in Rust.

## Aims

#### Small
akimbo has a hard upper limit of 1000 lines of code, excluding blank lines and comments. Stats for each version are included below.

#### PST Only
akimbo's evaluation consists only of a set of tapered piece-square tables, tuned using Texel's tuning method.

## Stats
|                           Version                                |     Release Date     | SLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/) |
| :---------------------------------------------------------------:|:--------------------:|:----:|:------:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1) |    8th January 2023  | 1167 |  2469  |
| [0.2.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.2.0) |   10th   April 2023  |  866 |  2525  |
| [0.3.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.3.0) |   16th     May 2023  |  891 |  2587  |
| [0.4.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.4.0) |    4th    July 2023  |  852 |   TBD  |

## Features

#### Evaluation
- Tapered Piece-Square Tables

#### Selectivity
- Aspiration Windows
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning (with Improving)
- Null Move Pruning
- Internal Iterative Reductions
- Late Move Pruning
- Razoring

#### Move Ordering
1. Hash Move
2. Captures (MVV-LVA)
3. Promotions
4. Killer Moves
5. History Heuristic (with Gravity)