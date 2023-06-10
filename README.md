<div align="center">

# akimbo

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/JacquesRW/akimbo?style=for-the-badge)](https://github.com/JacquesRW/akimbo/releases/latest)

</div>

A compact UCI Chess and Chess960 engine written in Rust.

The main aim is to stay under 1000 SLOC (excluding comments and blank lines) and 1500 TLOC (including the aforementioned).

## Stats
|                           Version                                |     Release Date     | SLOC | TLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/) | [CCRL 40/2 FRC](https://www.computerchess.org.uk/ccrl/404FRC/) |
| :---------------------------------------------------------------:|:--------------------:|:----:|:----:|:-------:|:------:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1) |    8th January 2023  | 1167 | 1381 |   2469  |  2313  |
| [0.2.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.2.0) |   10th   April 2023  |  866 | 1000 |   2525  |   N/A  |
| [0.3.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.3.0) |   16th     May 2023  |  891 | 1050 |  ~2587  |   TBD  |

## Features

#### Evaluation
- Tapered Piece-Square Tables

#### Selectivity
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
- Razoring
- Internal Iterative Reductions

#### Move Ordering
1. Hash Move
2. Captures (MVV-LVA)
3. Promotions
4. Killer Moves
5. History Heuristic
