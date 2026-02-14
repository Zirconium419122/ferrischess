use std::{collections::HashSet, time::Instant};

use chessframe::{
    bitboard::{BitBoard, EMPTY}, board::Board, chess_move::ChessMove, color::Color, piece::Piece, square::Square, transpositiontable::TranspositionTable
};

use crate::{eval::Eval, move_sorter::MoveSorter};

// Let's just use 1 billion instead of i32::MAX since I'm scared of overflow and underflow.
pub const INFINITY: i32 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Default)]
pub enum Bound {
    #[default]
    None,
    Exact,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Default)]
pub enum TimeManagement {
    #[default]
    None,
    MoveTime { time: usize },
    TimeLeft { time: usize },
}

impl TimeManagement {
    pub fn new(
        move_time: Option<usize>,
        time: Option<usize>,
        time_inc: Option<usize>,
    ) -> TimeManagement {
        if let Some(move_time) = move_time {
            TimeManagement::MoveTime {
                time: move_time.max(5),
            }
        } else if let Some(time) = time {
            TimeManagement::TimeLeft {
                time: (time / 20 + time_inc.unwrap_or(0) / 2).max(5),
            }
        } else {
            TimeManagement::None
        }
    }

    pub fn should_cancel_search(&self, search: &mut Search) -> bool {
        if search.think_timer.elapsed().as_millis() as usize >= self.time()
            && *self != TimeManagement::None
        {
            search.cancelled = true;
        }
        search.cancelled
    }

    pub fn time(&self) -> usize {
        match self {
            TimeManagement::MoveTime { time } | TimeManagement::TimeLeft { time } => *time,
            _ => 0,
        }
    }
}

pub struct Search<'a> {
    board: &'a Board,
    search_depth: u8,

    repetition_table: HashSet<u64>,
    transposition_table: &'a mut TranspositionTable<(i32, Bound, ChessMove)>,
    move_sorter: &'a mut MoveSorter,

    evaluation: i32,
    best_move: ChessMove,
    pv: Vec<ChessMove>,

    evaluation_iteration: i32,
    best_move_iteration: ChessMove,
    pv_iteration: Vec<ChessMove>,

    pub nodes: usize,
    pub seldepth: u8,

    think_timer: Instant,
    pub time_management: TimeManagement,

    cancelled: bool,
}

pub struct SearchInfo {
    pub depth: Option<usize>,
    pub seldepth: Option<usize>,

    pub time: Option<usize>,
    pub nodes: Option<usize>,
    pub nps: Option<usize>,

    pub evaluation: Option<isize>,
    pub best_move: Option<ChessMove>,
    pub pv: Option<Vec<ChessMove>>,
}

impl<'a> Search<'a> {
    pub const NULL_MOVE: ChessMove = ChessMove { from: Square::A1, to: Square::A1, promotion: None };

    pub const MAX_PLY: u8 = 255;

    pub fn new(
        board: &'a Board,
        depth: Option<u8>,
        time_management: TimeManagement,
        repetition_table: HashSet<u64>,
        transposition_table: &'a mut TranspositionTable<(i32, Bound, ChessMove)>,
        move_sorter: &'a mut MoveSorter,
    ) -> Search<'a> {
        Search {
            board,
            search_depth: depth.unwrap_or(Search::MAX_PLY),

            repetition_table,
            transposition_table,
            move_sorter,

            evaluation: 1234567890,
            best_move: Search::NULL_MOVE,
            pv: Vec::new(),

            evaluation_iteration: 1234567890,
            best_move_iteration: Search::NULL_MOVE,
            pv_iteration: Vec::new(),

            nodes: 0,
            seldepth: 0,

            think_timer: Instant::now(),
            time_management,

            cancelled: false,
        }
    }

    pub fn start_search(&mut self) -> SearchInfo {
        let search_depth = if self.time_management != TimeManagement::None {
            Search::MAX_PLY
        } else {
            self.search_depth
        };

        self.cancelled = false;
        self.best_move = Search::NULL_MOVE;

        const WINDOWS: [i32; 3] = [
            15,
            350,
            INFINITY
        ];

        let mut evaluation = 0;

        let mut depth_searched = 0;

        self.think_timer = Instant::now();
        for depth in 1..=search_depth {
            self.search_depth = depth;

            let mut tries = 1;

            let (mut alpha, mut beta) = if depth >= 6 {
                (evaluation - WINDOWS[0], evaluation + WINDOWS[0])
            } else {
                (-INFINITY, INFINITY)
            };

            loop {
                (self.evaluation_iteration, self.best_move_iteration) =
                    self.search_base(alpha, beta);

                evaluation = self.evaluation_iteration;

                if evaluation <= alpha && tries < WINDOWS.len() - 1 {
                    alpha = evaluation.saturating_sub(WINDOWS[tries]);
                    tries += 1;

                    continue;
                }
                if evaluation >= beta && tries < WINDOWS.len() - 1 {
                    beta = evaluation.saturating_add(WINDOWS[tries]);
                    tries += 1;

                    continue;
                }

                if self.best_move_iteration != Search::NULL_MOVE {
                    self.pv = self.pv_iteration.clone();
                    self.evaluation = self.evaluation_iteration;
                    self.best_move = self.best_move_iteration;

                    depth_searched = self.search_depth;
                }

                break;
            }

            if self.cancelled {
                break;
            }
        }

        let nodes = self.nodes;
        let elapsed = self.think_timer.elapsed().as_millis() as usize;

        SearchInfo {
            depth: Some(depth_searched as usize),
            seldepth: Some(self.seldepth as usize),
            time: Some(elapsed),
            nodes: Some(nodes),
            nps: Some((nodes as f32 * 1000.0 / elapsed as f32).round() as usize),
            evaluation: Some(self.evaluation as isize),
            best_move: Some(self.best_move),
            pv: Some(self.pv.clone()),
        }
    }

    pub fn should_cancel_search(&mut self) -> bool {
        let time_management = self.time_management;
        time_management.should_cancel_search(self)
    }

    pub fn search_base(&mut self, mut alpha: i32, beta: i32) -> (i32, ChessMove) {
        let mut legal_moves = false;
        let mut max = i32::MIN;
        let mut best_move = Search::NULL_MOVE;

        let mut inserted = false;
        let zobrist_hash = self.board.hash();
        if !self.repetition_table.contains(&zobrist_hash) {
            inserted = true;
            self.repetition_table.insert(zobrist_hash);
        }

        let first_move = self
            .transposition_table
            .get(self.board.hash())
            .map(|entry| entry.value.2);

        let mut moves = self.board.generate_moves_vec(!EMPTY);
        self.move_sorter.sort_moves(self.board, &mut moves, first_move, self.pv.first().copied(), 1);
        for mv in moves {
            if let Ok(board) = self.board.make_move_new(&mv) {
                let mut base_pv = Vec::new();

                legal_moves = true;
                let score = -self.search(&board, -beta, -alpha, self.search_depth - 1, 1, &mut base_pv);

                if self.should_cancel_search() {
                    if best_move != Search::NULL_MOVE {
                        break;
                    } else {
                        let _ = self.repetition_table.remove(&zobrist_hash);
                        return (0, Search::NULL_MOVE);
                    }
                }

                if score > max {
                    max = score;
                    best_move = mv;

                    if score > alpha {
                        alpha = score;

                        self.pv_iteration.clear();
                        self.pv_iteration.push(mv);
                        self.pv_iteration.append(&mut base_pv);
                    }
                }
            }
        }

        if inserted {
            let _ = self.repetition_table.remove(&zobrist_hash);
        }

        if !legal_moves {
            if self.board.in_check() {
                return (-Eval::MATE_SCORE, Search::NULL_MOVE);
            } else {
                return (0, Search::NULL_MOVE);
            }
        }

        (max, best_move)
    }

    fn search(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        mut depth: u8,
        ply: u8,
        pv: &mut Vec<ChessMove>,
    ) -> i32 {
        if self.should_cancel_search() {
            return 0;
        }

        if board.in_check() {
            depth += 1;
        }

        if depth == 0 {
            return self.search_captures(board, alpha, beta, ply);
        }

        self.nodes += 1;

        let zobrist_hash = board.hash();
        if !self.repetition_table.contains(&zobrist_hash) {
            self.repetition_table.insert(zobrist_hash);
        } else {
            return 0;
        }

        let original_alpha = alpha;
        let mut legal_moves = false;
        let mut max = i32::MIN;
        let mut best_move = None;

        let entry = self.transposition_table.get(zobrist_hash);

        if let Some(entry) = entry {
            if entry.depth >= depth {
                let corrected_score = Self::correct_mate_score(entry.value.0, ply);
                match entry.value.1 {
                    Bound::Exact => return corrected_score,
                    // Bound::Lower if corrected_score >= beta => return corrected_score,
                    Bound::Upper if corrected_score <= alpha => return corrected_score,
                    _ => {}
                }
            }
        }

        let mut moves = board.generate_moves_vec(!EMPTY);
        self.move_sorter.sort_moves(board, &mut moves, entry.map(|entry| entry.value.2), self.pv.get(ply as usize - 1).copied(), ply);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                let mut node_pv = Vec::with_capacity(8);

                legal_moves = true;
                let score = -self.search(&board, -beta, -alpha, depth.saturating_sub(1), ply + 1, &mut node_pv);

                if score > max {
                    max = score;
                    best_move = Some(mv);

                    if score > alpha {
                        alpha = score;

                        pv.clear();
                        pv.push(mv);
                        pv.append(&mut node_pv);
                    }
                }
                if score >= beta {
                    self.transposition_table.store(
                        board.hash(),
                        (score, Bound::Lower, mv),
                        depth,
                    );
                    let _ = self.repetition_table.remove(&zobrist_hash);

                    self.move_sorter.add_killer_move(mv, ply);

                    return beta;
                }
            }
        }

        let _ = self.repetition_table.remove(&zobrist_hash);

        if !legal_moves {
            if board.in_check() {
                return -Eval::MATE_SCORE + ply as i32;
            } else {
                return 0;
            }
        }

        if let Some(best_move) = best_move {
            if alpha <= original_alpha {
                self.transposition_table.store(
                    zobrist_hash,
                    (max, Bound::Upper, best_move),
                    depth,
                );
            } else if alpha >= beta {
                self.transposition_table.store(
                    zobrist_hash,
                    (max, Bound::Exact, best_move),
                    depth,
                );
            }
        }

        max
    }

    fn search_captures(&mut self, board: &Board, mut alpha: i32, beta: i32, ply: u8) -> i32 {
        let eval = Eval::new(board).eval();
        if eval >= beta {
            return eval;
        }
        if eval > alpha {
            alpha = eval;
        }

        self.seldepth = self.seldepth.max(ply);

        self.nodes += 1;

        let mut moves = board.generate_moves_vec(board.occupancy(!board.side_to_move));
        self.move_sorter.sort_moves(board, &mut moves, None, self.pv.get(ply as usize - 1).copied(), ply);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                let score = -self.search_captures(&board, -beta, -alpha, ply + 1);

                if score >= beta {
                    return score;
                }
                if score > alpha {
                    alpha = score;
                }
            }
        }

        alpha
    }

    fn pawn_attack_mask(board: &Board, color: Color) -> BitBoard {
        match color {
            Color::White => {
                ((board.pieces_color(Piece::Pawn, color) << 7) & !BitBoard(0x8080808080808080))
                    | ((board.pieces_color(Piece::Pawn, color) << 9)
                        & !BitBoard(0x1010101010101010))
            }
            Color::Black => {
                ((board.pieces_color(Piece::Pawn, color) >> 7) & !BitBoard(0x1010101010101010))
                    | ((board.pieces_color(Piece::Pawn, color) >> 9)
                        & !BitBoard(0x8080808080808080))
            }
        }
    }

    fn correct_mate_score(score: i32, ply: u8) -> i32 {
        if score.abs() > Eval::MATE_SCORE - 1000 {
            let sign = score.signum();
            return (score * sign - ply as i32) * sign;
        }
        score
    }
}
