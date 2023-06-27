<div align="center">

# akimbo

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/JacquesRW/akimbo?style=for-the-badge)](https://github.com/JacquesRW/akimbo/releases/latest)

</div>

A compact UCI Chess engine written in Rust.

The main aim is to stay under 1000 SLOC (excluding comments and blank lines) and 1500 TLOC (including the aforementioned).

## Stats
|                           Version                                |     Release Date     | SLOC | TLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/) |
| :---------------------------------------------------------------:|:--------------------:|:----:|:----:|:------:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1) |    8th January 2023  | 1167 | 1381 |  2469  |
| [0.2.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.2.0) |   10th   April 2023  |  866 | 1000 |  2525  |
| [0.3.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.3.0) |   16th     May 2023  |  891 | 1050 |  2587  |

## Features

#### Evaluation
- Tapered Midgame to Endgame
- Piece-Square Tables
- Passed Pawn Bonus by Rank

#### Selectivity
- Aspiration Windows
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning (with Improving)
- Null Move Pruning
- Razoring
- Internal Iterative Reductions

#### Move Ordering
1. Hash Move
2. Captures (MVV-LVA)
3. Promotions
4. Killer Moves
5. History Heuristic
