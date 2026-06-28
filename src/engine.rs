use std::{
    collections::HashSet,
    io,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use chessframe::{board::Board, color::Color, uci::*};

use crate::{
    move_sorter::MoveSorter, search::Search, time_management::TimeManagement,
    transposition_table::TranspositionTable,
};

pub struct Engine {
    board: Board,
    repetition_table: Vec<u64>,
    transposition_table: Arc<TranspositionTable>,
    move_sorter: Arc<Mutex<MoveSorter>>,
    cancelled: Arc<AtomicBool>,
    quitting: bool,
}

impl Uci for Engine {
    fn send_command(&mut self, command: UciCommand) {
        match command {
            UciCommand::Id { name, author } => {
                println!("id name {}", name);
                println!("id author {}", author);
            }
            UciCommand::UciOk => {
                println!("uciok");
            }
            UciCommand::ReadyOk => {
                println!("readyok");
            }
            UciCommand::BestMove { best_move, ponder } => {
                if let Some(ponder) = ponder {
                    println!("bestmove {} ponder {}", best_move, ponder);
                } else {
                    println!("bestmove {}", best_move);
                }
            }
            UciCommand::Info(info) => {
                println!("{}", info);
            }
            _ => {}
        }
    }

    fn read_command(&mut self) -> Option<UciCommand> {
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();

        UciCommand::from_str(line.trim()).ok()
    }

    fn handle_command(&mut self) {
        if let Some(command) = self.read_command() {
            match command {
                UciCommand::Uci => {
                    self.send_command(UciCommand::Id {
                        name: "Ferrischess".to_string(),
                        author: "Zirconium419122".to_string(),
                    });
                    self.send_command(UciCommand::UciOk);
                }
                UciCommand::Debug(debug) => {
                    if debug {
                        self.send_command(UciCommand::Info(Info {
                            string: Some("Debug mode not supported yet!".to_string()),
                            ..Default::default()
                        }));
                    }
                }
                UciCommand::IsReady => self.send_command(UciCommand::ReadyOk),
                UciCommand::UciNewGame => {
                    self.board = Board::default();
                    self.repetition_table.clear();
                    self.move_sorter.lock().unwrap().clear();
                }
                UciCommand::Position { fen, moves } => {
                    if fen == "startpos" {
                        self.board = Board::default();
                    } else {
                        self.board = Board::from_fen(&fen);
                    };
                    self.repetition_table.clear();

                    if let Some(moves) = moves {
                        let board = &mut self.board;

                        for mv in moves {
                            let zobrist_hash = board.hash();
                            self.repetition_table.push(zobrist_hash);

                            let mv = board.infer_move(&mv).unwrap();

                            let _ = board.make_move(mv);
                        }
                    }
                }
                UciCommand::Go(Go {
                    depth,
                    wtime,
                    winc,
                    btime,
                    binc,
                    move_time,
                    ..
                }) => {
                    self.cancelled.store(false, Ordering::Relaxed);

                    let mut repetition_table = HashSet::from_iter(self.repetition_table.clone());
                    repetition_table.reserve(16);
                    let transposition_table = self.transposition_table.clone();
                    let move_sorter = self.move_sorter.clone();

                    let (time, time_inc) = if self.board.side_to_move == Color::White {
                        (wtime, winc)
                    } else {
                        (btime, binc)
                    };

                    let board = self.board;
                    let cancelled = self.cancelled.clone();

                    thread::spawn(move || {
                        let mut search = Search::new(
                            board,
                            depth.map(|depth| depth as u8),
                            TimeManagement::new(move_time, time, time_inc),
                            repetition_table,
                            transposition_table,
                            move_sorter,
                            cancelled,
                        );

                        search.start_search();
                    });
                }
                UciCommand::Stop => self.cancelled.store(true, Ordering::Relaxed),
                UciCommand::Quit => self.quitting = true,
                _ => {}
            }
        }
    }
}

impl Engine {
    const TRANSPOSITIONTABLE_SIZE: usize = 64;

    pub fn new() -> Engine {
        Engine {
            board: Board::default(),
            repetition_table: Vec::new(),
            transposition_table: Arc::new(TranspositionTable::with_size_mb(
                Engine::TRANSPOSITIONTABLE_SIZE,
            )),
            move_sorter: Arc::new(Mutex::new(MoveSorter::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
            quitting: false,
        }
    }

    pub fn run(&mut self) {
        loop {
            self.handle_command();

            if self.quitting {
                break;
            }
        }
    }
}
