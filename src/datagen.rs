use crate::{
    consts::Side,
    frc::Castling,
    moves::Move,
    pos::Position,
    search::go,
    tables::{HashTable, HistoryTable},
    thread::ThreadData,
    util::STARTPOS,
};

// Datagen Settings
const ADJ_WIN_SCORE: i32 = 3000;
const SOFT_NODE_LIMIT: u64 = 5_000;
const HARD_NODE_LIMIT: u64 = 1_000_000;
const HARD_TIMEOUT_MS: u128 = 10_000;
const DATA_WRITE_RATE: u64 = 16;

use bulletformat::{BulletFormat, ChessBoard};

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

static STOP: AtomicBool = AtomicBool::new(false);

pub fn run_datagen(threads: usize, tcp_ip: Option<&str>, book_path: Option<&str>) {
    let tcp_ip = tcp_ip.map(|ip| {
        println!("#[Connecting] {ip}");
        TcpStream::connect(ip).expect("Couldn't connect.")
    });

    let book = book_path.map(|path| {
        let file = BufReader::new(File::open(path).unwrap());
        file.lines()
            .collect::<Vec<Result<String, std::io::Error>>>()
    });

    std::thread::scope(|s| {
        for num in 0..threads {
            let bref = &book;
            std::thread::sleep(std::time::Duration::from_millis(10));
            let this_ip = tcp_ip
                .as_ref()
                .map(|ip| ip.try_clone().expect("Couldn't Clone!"));
            s.spawn(move || {
                let mut worker = DatagenThread::new(SOFT_NODE_LIMIT, 8, num, this_ip);
                worker.run_datagen(bref);
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
            Destination::BinFile(BufWriter::new(
                File::create(format!("resources/akimbo-{seed}.data")).unwrap(),
            ))
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

    fn run_datagen(&mut self, book: &Option<Vec<Result<String, std::io::Error>>>) {
        let mut tt = HashTable::default();
        tt.resize(self.hash_size, 1);

        let mut data = Vec::new();

        loop {
            if STOP.load(SeqCst) {
                break;
            }

            let optional = self.run_game(&tt, book);
            tt.clear(1);

            let result = if let Some(res) = optional {
                res
            } else {
                continue;
            };

            self.games += 1;

            for &(bbs, stm, score) in result.fens.iter() {
                let board = ChessBoard::from_raw(bbs, stm, score, result.result).unwrap();
                data.push(board);
                self.fens += 1;
            }

            if self.games % DATA_WRITE_RATE == 0 {
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
                stream
                    .write_all(buf)
                    .unwrap_or_else(|_| panic!("#[{}] error writing to stream", self.id))
            }
        }

        data.clear();
    }

    fn run_game(
        &mut self,
        tt: &HashTable,
        book: &Option<Vec<Result<String, std::io::Error>>>,
    ) -> Option<GameResult> {
        let mut castling = Castling::default();
        let mut pos = if let Some(list) = book {
            let rng = self.rng() as usize % list.len();
            Position::from_fen(list[rng].as_ref().unwrap(), &mut castling)
        } else {
            Position::from_fen(STARTPOS, &mut castling)
        };

        let abort = AtomicBool::new(false);
        let mut engine = ThreadData {
            max_nodes: HARD_NODE_LIMIT,
            max_time: HARD_TIMEOUT_MS,
            ..ThreadData::new(&abort, tt, Vec::new(), HistoryTable::default(), castling)
        };

        // play 8 or 9 random moves
        for _ in 0..(8 + (self.rng() % 2)) {
            let moves = pos.movegen::<true>(&castling);
            let mut legals = Vec::new();
            for &mov in moves.iter() {
                let mut new = pos;
                if !new.make(mov, &castling) {
                    legals.push(mov);
                }
            }

            if legals.is_empty() {
                return None;
            }

            engine.stack.push(pos.hash());
            pos.make(legals[self.rng() as usize % legals.len()], &castling);
        }

        let mut result = GameResult::default();

        // play out game
        loop {
            let (bm, score) = go(&pos, &mut engine, false, 32, 1000.0, self.nodes_per_move);

            engine.tt.age_up();

            // adjudicate large scores
            if score.abs() > ADJ_WIN_SCORE {
                result.result = if score > 0 { 1 - pos.stm() } else { pos.stm() } as f32;

                break;
            }

            // pos is quiet, can use fen
            if !bm.is_capture() && !pos.in_check() {
                let wscore = if pos.stm() == Side::BLACK {
                    -score
                } else {
                    score
                };
                result
                    .fens
                    .push((pos.bitboards(), pos.stm(), wscore as i16));
            }

            // not enough nodes to finish a depth!
            engine.stack.push(pos.hash());
            if bm == Move::NULL || pos.make(bm, &castling) {
                return None;
            }

            // check for game end via check/stalemate
            if is_terminal(&pos, &castling) {
                result.result = if pos.in_check() {
                    pos.stm() as f32
                } else {
                    0.5
                };
                break;
            }

            // check for game end via other draw rules
            if pos.draw() || engine.repetition(&pos, pos.hash(), true) {
                result.result = 0.5;
                break;
            }
        }

        Some(result)
    }
}

fn is_terminal(pos: &Position, castling: &Castling) -> bool {
    let moves = pos.movegen::<true>(castling);
    for &mov in moves.iter() {
        let mut new = *pos;
        if !new.make(mov, castling) {
            return false;
        }
    }
    true
}
