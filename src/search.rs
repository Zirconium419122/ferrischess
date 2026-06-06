use std::{collections::HashSet, time::Instant};

use chessframe::{
    bitboard::EMPTY, board::Board, chess_move::ChessMove, piece::Piece,
    transpositiontable::TranspositionTable,
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
            best_move: ChessMove::NULL_MOVE,
            pv: Vec::new(),

            evaluation_iteration: 1234567890,
            best_move_iteration: ChessMove::NULL_MOVE,
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
        self.best_move = ChessMove::NULL_MOVE;

        let mut evaluation = 0;

        let mut depth_searched = 0;

        self.think_timer = Instant::now();
        for depth in 1..=search_depth {
            self.search_depth = depth;

            let mut delta = 16;

            let (mut alpha, mut beta) = if depth >= 6 {
                (evaluation - delta, evaluation + delta)
            } else {
                (-INFINITY, INFINITY)
            };

            loop {
                (self.evaluation_iteration, self.best_move_iteration) =
                    self.search_base(alpha, beta);

                evaluation = self.evaluation_iteration;

                if evaluation <= alpha {
                    alpha = evaluation.saturating_sub(delta);
                    delta += delta / 3;

                    continue;
                }
                if evaluation >= beta {
                    beta = evaluation.saturating_add(delta);
                    delta += delta / 3;

                    continue;
                }

                if self.best_move_iteration != ChessMove::NULL_MOVE {
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
        let mut best_move = ChessMove::NULL_MOVE;

        let mut inserted = false;
        let zobrist_hash = self.board.hash();
        if !self.repetition_table.contains(&zobrist_hash) {
            inserted = self.repetition_table.insert(zobrist_hash);
        }

        let original_alpha = alpha;

        self.move_sorter.age_history();

        let first_move = self
            .transposition_table
            .get(self.board.hash())
            .map(|entry| entry.value.2);

        let mut moves = self.board.generate_moves_vec(!EMPTY);
        self.move_sorter.sort_moves(self.board, &mut moves, first_move, self.pv.first().copied(), 1);
        for mv in moves {
            if let Ok(board) = self.board.make_move_new(mv) {
                let mut base_pv = [ChessMove::NULL_MOVE; 16];

                legal_moves = true;
                let score = -self.search(&board, -beta, -alpha, self.search_depth - 1, 1, &mut base_pv);

                if self.should_cancel_search() {
                    if best_move != ChessMove::NULL_MOVE {
                        break;
                    } else {
                        let _ = self.repetition_table.remove(&zobrist_hash);
                        return (0, ChessMove::NULL_MOVE);
                    }
                }

                if score > max {
                    max = score;
                    best_move = mv;

                    if score > alpha {
                        alpha = score;

                        self.pv_iteration.clear();
                        self.pv_iteration.push(mv);
                        self.pv_iteration.extend_from_slice(&base_pv);
                        self.pv_iteration.retain(|mv| *mv != ChessMove::NULL_MOVE);
                    }
                }
            }
        }

        if inserted {
            self.repetition_table.remove(&zobrist_hash);
        }

        if !legal_moves {
            if self.board.in_check() {
                return (-Eval::MATE_SCORE, ChessMove::NULL_MOVE);
            } else {
                return (0, ChessMove::NULL_MOVE);
            }
        }

        if max <= original_alpha {
            self.transposition_table.store(
                zobrist_hash,
                (max, Bound::Upper, best_move),
                self.search_depth,
            );
        } else if max >= beta {
            self.transposition_table.store(
                zobrist_hash,
                (max, Bound::Lower, best_move),
                self.search_depth,
            );
        } else {
            self.transposition_table.store(
                zobrist_hash,
                (max, Bound::Exact, best_move),
                self.search_depth,
            );
        }

        (max, best_move)
    }

    fn search(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        depth: u8,
        ply: u8,
        pv: &mut [ChessMove],
    ) -> i32 {
        if depth == 0 {
            return self.search_captures(board, alpha, beta, ply);
        }

        self.nodes += 1;

        let zobrist_hash = board.hash();

        let inserted = if !self.repetition_table.contains(&zobrist_hash) {
            self.repetition_table.insert(zobrist_hash)
        } else {
            return 0;
        };

        let original_alpha = alpha;
        let mut legal_moves = false;
        let mut max = i32::MIN;
        let mut best_move = None;

        let entry = self.transposition_table.get(zobrist_hash).copied();

        if !board.in_check()
            && (board.occupancy(board.side_to_move)
                ^ board.pieces_color(Piece::Pawn, board.side_to_move))
            .count_ones()
                != 1
        {
            if let Ok(board) = board.make_null_move_new() {
                let mut node_pv = [ChessMove::NULL_MOVE; 16];

                let score = -self.search(&board, -beta, -(beta - 1), depth.saturating_sub(3), ply + 1, &mut node_pv);

                if score >= beta {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }
                    return score;
                }
            }
        }

        if let Some(entry) = entry
            && entry.depth >= depth
        {
            let corrected_score = Self::correct_mate_score(entry.value.0, ply);

            match entry.value.1 {
                Bound::Exact => {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }
                    return corrected_score;
                }
                // Bound::Lower if corrected_score >= beta => return corrected_score,
                Bound::Upper if corrected_score <= alpha => {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }
                    return corrected_score;
                }
                _ => {}
            }
        }

        let mut quiets = Vec::with_capacity(8);

        let mut moves = board.generate_moves_vec(!EMPTY);
        self.move_sorter.sort_moves(board, &mut moves, entry.map(|entry| entry.value.2), self.pv.get(ply as usize - 1).copied(), ply);
        for mv in moves {
            if let Ok(node_board) = board.make_move_new(mv) {
                let mut node_pv = [ChessMove::NULL_MOVE; 16];

                legal_moves = true;
                let score = -self.search(&node_board, -beta, -alpha, depth - 1 + node_board.in_check() as u8, ply + 1, &mut node_pv);

                let is_quiet = !board.combined().is_set(mv.to);
                if is_quiet {
                    quiets.push(mv);
                }

                if score > max {
                    max = score;
                    best_move = Some(mv);

                    if score > alpha {
                        alpha = score;

                        pv[0] = mv;
                        pv[1..].copy_from_slice(&node_pv[..15]);
                    }
                }
                if score >= beta {
                    self.transposition_table.store(
                        zobrist_hash,
                        (score, Bound::Lower, mv),
                        depth,
                    );
                    if inserted { self.repetition_table.remove(&zobrist_hash); }

                    if is_quiet {
                        self.move_sorter.update_history(
                            mv.to,
                            unsafe { board.get_piece(mv.from).unwrap_unchecked() },
                            depth as i16 * depth as i16,
                        );

                        quiets.pop();
                        for quiet in quiets {
                            self.move_sorter.update_history(
                                quiet.to,
                                unsafe { board.get_piece(quiet.from).unwrap_unchecked() },
                                -(depth as i16),
                            );
                        }

                        self.move_sorter.add_killer_move(mv, ply);
                    }

                    return beta;
                }

                if self.should_cancel_search() {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }

                    return max;
                }
            }
        }

        if inserted { self.repetition_table.remove(&zobrist_hash); }

        if !legal_moves {
            if board.in_check() {
                return -Eval::MATE_SCORE + ply as i32;
            } else {
                return 0;
            }
        }

        if let Some(best_move) = best_move {
            if max <= original_alpha {
                self.transposition_table.store(
                    zobrist_hash,
                    (max, Bound::Upper, best_move),
                    depth,
                );
            } else {
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
        self.seldepth = self.seldepth.max(ply);
        self.nodes += 1;

        let eval = Eval::new(board).eval();
        if eval >= beta {
            return eval;
        }
        if eval > alpha {
            alpha = eval;
        }

        let mut moves = board.generate_moves_vec(board.occupancy(!board.side_to_move));
        self.move_sorter.sort_moves(board, &mut moves, None, self.pv.get(ply as usize - 1).copied(), ply);
        for mv in moves {
            if let Ok(board) = board.make_move_new(mv) {
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

    fn correct_mate_score(score: i32, ply: u8) -> i32 {
        if score.abs() > Eval::MATE_SCORE - 1000 {
            let sign = score.signum();
            return (score * sign - ply as i32) * sign;
        }
        score
    }
}
