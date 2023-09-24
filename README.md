<div align="center">

# akimbo

![License](https://img.shields.io/github/license/jw1912/akimbo?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jw1912/akimbo?style=for-the-badge)](https://github.com/jw1912/akimbo/releases/latest)
[![Commits](https://img.shields.io/github/commits-since/jw1912/akimbo/latest?style=for-the-badge)](https://github.com/jw1912/akimbo/commits/main)

</div>

A compact UCI Chess engine written in Rust.

Ending with version 0.6.0, akimbo had a hard upper limit of 1000 lines of code, excluding blank lines and comments.
Stats are included below (SLOC), but this restraint is now abandoned in favour of making a more feature-complete engine.

Huge thanks to all of the members of [this OpenBench Instance](https://chess.swehosting.se/users/) who have provided support and guidance in the development
of akimbo, as well as facilitating far faster testing than on my own.

## Evaluation
All data used is self-generated, starting from material values when akimbo still had an HCE and iteratively generating data and tuning to
produce higher quality datasets. The final HCE dataset was then used to train akimbo's first network and further data has been generated
since.

Additionally, akimbo uses its own trainer written in Rust and CUDA, [bullet](https://github.com/jw1912/bullet).

## Stats
<div align="center">

|                           Version                                |     Release Date     | SLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/) | [CCRL Blitz](http://ccrl.chessdom.com/ccrl/404/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on&num_best_in_class=1&print=Rating+list&profile_step=50&profile_numbers=1&print=Results+table&print=LOS+table&table_size=100&ct_from_elo=0&ct_to_elo=10000&match_length=30&cross_tables_for_best_versions_only=1&sort_tables=by+rating&diag=0&reference_list=None&recalibrate=no) | Notes |
| :------------------------------------------------------------:|:-------------------:|:----:|:----:|:----:|:------------------------:|
| [0.1.1](https://github.com/jw1912/akimbo/releases/tag/v0.1.1) |  8th   January 2023 | 1167 | 2469 |  -   |      First Release       |
| [0.2.0](https://github.com/jw1912/akimbo/releases/tag/v0.2.0) | 10th     April 2023 |  866 | 2524 |  -   |           N/A            |
| [0.3.0](https://github.com/jw1912/akimbo/releases/tag/v0.3.0) | 16th       May 2023 |  891 | 2587 |  -   |           N/A            |
| [0.4.0](https://github.com/jw1912/akimbo/releases/tag/v0.4.0) |  4th      July 2023 |  852 | 2725 | 2760 |           N/A            |
| [0.4.1](https://github.com/jw1912/akimbo/releases/tag/v0.4.1) |  4th    August 2023 |  948 |  -   | 2866 |  Final PST-only Release  |
| [0.5.0](https://github.com/jw1912/akimbo/releases/tag/v0.5.0) | 13th    August 2023 |  940 | 3001 | 3069 |         Better HCE       |
| [0.6.0](https://github.com/jw1912/akimbo/releases/tag/v0.6.0) | 24th September 2023 |  898 | TBD  | TBD  | `768 -> 256x2 -> 1` NNUE |

</div>

## Compiling
Run the following command
```
cargo rustc --release --package akimbo --bin akimbo -- -C target-cpu=native
```
and the executable will be located in `target/release`.
