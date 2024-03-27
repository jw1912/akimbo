<div align="center">

# akimbo

![License](https://img.shields.io/github/license/jw1912/akimbo?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jw1912/akimbo?style=for-the-badge)](https://github.com/jw1912/akimbo/releases/latest)
[![Commits](https://img.shields.io/github/commits-since/jw1912/akimbo/latest?style=for-the-badge)](https://github.com/jw1912/akimbo/commits/main)

</div>

A strong UCI Chess and Chess960 engine written in Rust.

Ending with version 0.6.0, akimbo had a hard upper limit of 1000 lines of code, excluding blank lines and comments.
Stats are included below (SLOC), but this restraint is now abandoned in favour of making a more feature-complete engine.

Huge thanks to all of the members of [this OpenBench Instance](https://chess.swehosting.se/users/) who have provided support and guidance in the development
of akimbo, as well as facilitating far faster testing than on my own.

## Evaluation
Up to and including version 1.0.0, all data used was self-generated, starting from material values when akimbo still had an HCE and iteratively generating data and tuning to
produce higher quality datasets. The final HCE dataset was then used to train akimbo's first network and further data was then generated.

This involved hundreds of hours of compute, and got incredibly tedious, so akimbo now uses data produced by Leela Chess Zero.

Additionally, akimbo uses its own trainer written in Rust and CUDA, [bullet](https://github.com/jw1912/bullet), which also used by a number of other engines.

## Stats
<div align="center">

|                           Version                                |     Release Date     | SLOC | [CCRL 40/15](https://www.computerchess.org.uk/ccrl/4040/) | [CCRL Blitz](http://ccrl.chessdom.com/ccrl/404/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on&num_best_in_class=1&print=Rating+list&profile_step=50&profile_numbers=1&print=Results+table&print=LOS+table&table_size=100&ct_from_elo=0&ct_to_elo=10000&match_length=30&cross_tables_for_best_versions_only=1&sort_tables=by+rating&diag=0&reference_list=None&recalibrate=no) | Notes |
| :------------------------------------------------------------:|:-------------------:|:----:|:----:|:----:|:---------------------------:|
| [0.1.1](https://github.com/jw1912/akimbo/releases/tag/v0.1.1) |  8th   January 2023 | 1167 | 2444 |  -   |       First Release         |
| [0.2.0](https://github.com/jw1912/akimbo/releases/tag/v0.2.0) | 10th     April 2023 |  866 | 2506 |  -   |             -               |
| [0.3.0](https://github.com/jw1912/akimbo/releases/tag/v0.3.0) | 16th       May 2023 |  891 | 2574 |  -   |             -               |
| [0.4.0](https://github.com/jw1912/akimbo/releases/tag/v0.4.0) |  4th      July 2023 |  852 | 2730 | 2722 |             -               |
| [0.4.1](https://github.com/jw1912/akimbo/releases/tag/v0.4.1) |  4th    August 2023 |  948 |  -   | 2840 |   Final PST-only Release    |
| [0.5.0](https://github.com/jw1912/akimbo/releases/tag/v0.5.0) | 13th    August 2023 |  940 | 3026 | 3056 |         Better HCE          |
| [0.6.0](https://github.com/jw1912/akimbo/releases/tag/v0.6.0) | 24th September 2023 |  898 | 3336 |  -   |           NNUE              |
| [0.7.0](https://github.com/jw1912/akimbo/releases/tag/v0.7.0) | 30th   October 2023 |  -   | 3390 | 3476 |     DFRC + SMP Support      |
| [0.8.0](https://github.com/jw1912/akimbo/releases/tag/v0.8.0) |  2nd   January 2024 |  -   | 3438 | 3540 |             -               |
| [1.0.0](https://github.com/jw1912/akimbo/releases/tag/v0.8.0) |  26th    March 2024 |  -   | TBD  | TBD  | Final Original Data Release |

</div>

## Compiling
Run the following command
```
make EVALFILE=resources/net.bin
```
and the executable will be located in `target/release`.
