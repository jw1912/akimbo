<div align="center">

# akimbo

![License](https://img.shields.io/github/license/JacquesRW/akimbo?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/JacquesRW/akimbo?style=for-the-badge)](https://github.com/JacquesRW/akimbo/releases/latest)
![Commits](https://img.shields.io/github/commits-since/JacquesRW/akimbo/latest?style=for-the-badge)

</div>

A compact UCI Chess engine written in Rust.

akimbo has a hard upper limit of 1000 lines of code, excluding blank lines and comments. Stats for each version are included below.
This only applies to the actual engine, tuning and datagen are not counted (as I could just stick them in a different repo).

Huge thanks to all of the members of [this OpenBench Instance](https://chess.swehosting.se/users/) who have provided support and guidance in the development
of akimbo, as well as facilitating far faster testing than on my own.


## Stats
|                           Version                                |     Release Date     | SLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/) | [CCRL Blitz](http://ccrl.chessdom.com/ccrl/404/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on&num_best_in_class=1&print=Rating+list&profile_step=50&profile_numbers=1&print=Results+table&print=LOS+table&table_size=100&ct_from_elo=0&ct_to_elo=10000&match_length=30&cross_tables_for_best_versions_only=1&sort_tables=by+rating&diag=0&reference_list=None&recalibrate=no) |
| :---------------------------------------------------------------:|:--------------------:|:----:|:------:|:----:|
| [0.1.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.1.1) |    8th January 2023  | 1167 |  2469  |  -   |
| [0.2.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.2.0) |   10th   April 2023  |  866 |  2525  |  -   |
| [0.3.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.3.0) |   16th     May 2023  |  891 |  2587  |  -   |
| [0.4.0](https://github.com/JacquesRW/akimbo/releases/tag/v0.4.0) |    4th    July 2023  |  852 |  2724  | 2760 |
| [0.4.1](https://github.com/JacquesRW/akimbo/releases/tag/v0.4.1) |    4th  August 2023  |  948 |   -    | TBD  |

## Features

#### Evaluation
- Tapered Piece-Square Tables

#### Selectivity
- Aspiration Windows
- Check Extensions
- Late Move Reductions
- Reverse Futility Pruning
- Null Move Pruning
- Internal Iterative Reductions
- Late Move Pruning
- Razoring
- Singular Extensions
- Static Exchange Evaluation
- Improving Heuristic

#### Move Ordering
1. Hash Move
2. Captures
3. Promotions
4. Killer Moves
5. Counter Moves
6. History Heuristic
