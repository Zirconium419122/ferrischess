use std::{collections::HashSet, io, str::FromStr};

use chessframe::{
    board::Board, chess_move::ChessMove, color::Color, transpositiontable::TranspositionTable,
    uci::*,
};

use crate::{
    eval::Eval,
    search::{Bound, Search, TimeManagement},
};

pub struct Engine {
    board: Board,
    quitting: bool,
    repetition_table: Vec<u64>,
    transposition_table: TranspositionTable<(i32, Bound, ChessMove)>,
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
                    self.transposition_table.clear();
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

                            let _ = board.make_move(&mv);
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
                    let mut repetition_table = HashSet::from_iter(self.repetition_table.clone());
                    repetition_table.reserve(16);
                    let transposition_table = &mut self.transposition_table;

                    let (time, time_inc) = if self.board.side_to_move == Color::White {
                        (wtime, winc)
                    } else {
                        (btime, binc)
                    };

                    let search_info;
                    {
                        let mut search = Search::new(
                            &self.board,
                            depth.map(|depth| depth as u8),
                            TimeManagement::new(move_time, time),
                            repetition_table,
                            transposition_table,
                        );

                        search_info = search.start_search(
                            time.unwrap_or(move_time.unwrap_or(0)),
                            time_inc.unwrap_or(0),
                        );
                    }
                    let pv = search_info
                        .pv
                        .unwrap()
                        .iter()
                        .map(|mv| mv.to_string())
                        .collect::<Vec<String>>()
                        .join(" ");

                    let score = search_info.evaluation.unwrap() as i32;

                    if search_info.best_move != Some(Search::NULL_MOVE) {
                        let score = if Eval::mate_score(score) {
                            let moves_to_mate = Eval::MATE_SCORE - score.abs();
                            let mate_in_moves = (moves_to_mate / 2) + 1;

                            Score {
                                mate: Some(score.signum() as isize * mate_in_moves as isize),
                                ..Default::default()
                            }
                        } else {
                            Score {
                                cp: Some(score as isize),
                                ..Default::default()
                            }
                        };

                        self.send_command(UciCommand::Info(Info {
                            depth: search_info.depth,
                            seldepth: search_info.seldepth,
                            pv: Some(pv),
                            score: Some(score),
                            time: search_info.time,
                            nodes: search_info.nodes,
                            nps: search_info.nps,
                            ..Default::default()
                        }));

                        self.send_command(UciCommand::BestMove {
                            best_move: search_info.best_move.unwrap().to_string(),
                            ponder: None,
                        });
                    }
                }
                UciCommand::Stop => {}
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
            quitting: false,
            repetition_table: Vec::new(),
            transposition_table: TranspositionTable::with_size_mb(Engine::TRANSPOSITIONTABLE_SIZE),
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
