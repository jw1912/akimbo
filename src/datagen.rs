use crate::{
    moves::Move,
    position::Position,
    search::go,
    thread::ThreadData,
    tables::{HashTable, HistoryTable},
    STARTPOS,
};

const SEND_RATE: u64 = 16;

use bulletformat::{BulletFormat, ChessBoard};

use std::{
    fs::File,
    io::{BufWriter, Write},
    net::TcpStream,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

static STOP: AtomicBool = AtomicBool::new(false);

const NODES_PER_MOVE: u64 = 5_000;

pub fn run_datagen(threads: usize, gpt: u64, tcp_ip: Option<&str>) {
    let startpos = Position::from_fen(STARTPOS);

    let tcp_ip = tcp_ip.map(|ip| {
        println!("#[Connecting] {ip}");
        TcpStream::connect(ip).expect("Couldn't connect.")
    });

    std::thread::scope(|s| {
        for num in 0..threads {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let this_ip = tcp_ip.as_ref().map(|ip| ip.try_clone().expect("Couldn't Clone!"));
            s.spawn(move || {
                let mut worker = DatagenThread::new(NODES_PER_MOVE, 8, num, this_ip);
                worker.run_datagen(gpt, startpos);
            });
        }

        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let commands = input.split_whitespace().collect::<Vec<_>>();
            if let Some(&"stop") = commands.first() {
                STOP.store(true, SeqCst);
                break;
            }
        }
    })
}

#[derive(Default)]
struct GameResult {
    fens: Vec<([u64; 8], usize, i16)>,
    result: f32,
}

enum Destination {
    BinFile(BufWriter<File>),
    TcpStream(TcpStream),
}

struct DatagenThread {
    hash_size: usize,
    id: usize,
    rng: u64,
    dest: Destination,
    games: u64,
    fens: u64,
    start_time: Instant,
    nodes_per_move: u64,
}

impl DatagenThread {
    fn new(nodes_per_move: u64, hash_size: usize, id: usize, tcp_ip: Option<TcpStream>) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Guaranteed increasing.")
            .as_micros() as u64
            & 0xFFFF_FFFF;

        let dest = if let Some(ip) = tcp_ip {
            Destination::TcpStream(ip.try_clone().unwrap())
        } else {
            Destination::BinFile(BufWriter::new(File::create(format!("resources/akimbo-{seed}.data")).unwrap()))
        };

        let res = Self {
            hash_size,
            id,
            rng: seed,
            dest,
            games: 0,
            fens: 0,
            start_time: Instant::now(),
            nodes_per_move,
        };

        println!("#[{}] created", res.id);
        res
    }

    fn rng(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn run_datagen(
        &mut self,
        max_games: u64,
        startpos: Position,
    ) {
        let mut tt = HashTable::default();
        tt.resize(self.hash_size, 1);

        let mut data = Vec::new();

        while self.games < max_games {
            if STOP.load(SeqCst) {
                break;
            }

            let optional = self.run_game(&tt, startpos);
            tt.clear(1);

            let result = if let Some(res) = optional {
                res
            } else {
                continue;
            };

            self.games += 1;
            let num_taken = result
                .fens
                .len()
                .saturating_sub(if result.result == 0.5 { 8 } else { 0 });

            for &(bbs, stm, score) in result.fens.iter().take(num_taken) {
                let board = ChessBoard::from_raw(bbs, stm, score, result.result).unwrap();
                data.push(board);
                self.fens += 1;
            }

            if self.games % SEND_RATE == 0 {
                self.write(&mut data);
            }
        }

        if !data.is_empty() {
            self.write(&mut data);
        }

        if let Destination::BinFile(file) = &mut self.dest {
            file.flush().unwrap();
        }
    }

    fn write(&mut self, data: &mut Vec<ChessBoard>) {
        println!(
            "#[{}] written games {} fens {} fens/sec {:.0}",
            self.id,
            self.games,
            self.fens,
            self.fens as f32 / self.start_time.elapsed().as_secs_f32(),
        );

        match &mut self.dest {
            Destination::BinFile(file) => ChessBoard::write_to_bin(file, data).unwrap(),
            Destination::TcpStream(stream) => {
                let buf = ChessBoard::as_bytes_slice(data);
                stream.write_all(buf).unwrap_or_else(|_| panic!("#[{}] error writing to stream", self.id))
            }
        }

        data.clear();
    }

    fn run_game(&mut self, tt: &HashTable, mut position: Position) -> Option<GameResult> {
        let abort = AtomicBool::new(false);
        let mut engine = ThreadData {
            mloop: false,
            max_nodes: 1_000_000,
            max_time: 10000,
            ..ThreadData::new(&abort, tt, Vec::new(), HistoryTable::default())
        };

        // play 8 or 9 random moves
        for _ in 0..(8 + (self.rng() % 2)) {
            let moves = position.movegen::<true>();
            let mut legals = Vec::new();
            for &mov in moves.iter() {
                let mut new = position;
                if !new.make(mov) {
                    legals.push(mov);
                }
            }

            if legals.is_empty() {
                return None;
            }

            engine.stack.push(position.hash());
            position.make(legals[self.rng() as usize % legals.len()]);
        }

        let mut result = GameResult::default();

        // play out game
        loop {
            let (bm, score) = go(
                &position,
                &mut engine,
                false,
                32,
                1000.0,
                self.nodes_per_move,
            );

            // adjudicate large scores
            if score.abs() > 1000 {
                result.result = if score > 0 {
                    1 - position.stm()
                } else {
                    position.stm()
                } as f32;

                break;
            }

            // position is quiet, can use fen
            if !bm.is_capture() && !position.in_check() {
                result.fens.push((position.bitboards(), position.stm(), score as i16));
            }

            // not enough nodes to finish a depth!
            engine.stack.push(position.hash());
            if bm == Move::NULL || position.make(bm) {
                return None;
            }

            // check for game end via check/stalemate
            if is_terminal(&position) {
                result.result = if position.in_check() {
                    position.stm() as f32
                } else {
                    0.5
                };
                break;
            }

            // check for game end via other draw rules
            if position.draw() || engine.repetition(&position, position.hash(), true) {
                result.result = 0.5;
                break;
            }
        }

        Some(result)
    }
}

fn is_terminal(pos: &Position) -> bool {
    let moves = pos.movegen::<true>();
    for &mov in moves.iter() {
        let mut new = *pos;
        if !new.make(mov) {
            return false;
        }
    }
    true
}
