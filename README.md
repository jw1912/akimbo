<div align="center">

# akimbo

![License](https://img.shields.io/github/license/JacquesRW/akimbo?style=for-the-badge)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/JacquesRW/akimbo?style=for-the-badge)](https://github.com/JacquesRW/akimbo/releases/latest)
![Commits](https://img.shields.io/github/commits-since/JacquesRW/akimbo/latest?style=for-the-badge)

</div>

A compact, PST-only UCI Chess engine written in Rust.

Huge thanks to all of the members of [this OpenBench Instance](https://chess.swehosting.se/users/) who have provided support and guidance in the development
of akimbo, as well as facilitating far faster testing than on my own.

## Aims

#### Small
akimbo has a hard upper limit of 1000 lines of code, excluding blank lines and comments. Stats for each version are included below.

#### PST Only
akimbo's evaluation consists only of a set of tapered piece-square tables, tuned using Texel's tuning method.

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
