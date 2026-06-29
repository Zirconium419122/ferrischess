use std::{
    collections::HashSet,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::AtomicBool,
    },
    time::Instant,
};

use chessframe::{
    bitboard::EMPTY,
    board::Board,
    chess_move::ChessMove,
    piece::Piece,
    uci::{Info, Score},
};

use crate::{
    eval::{Eval, PIECE_VALUES_EG},
    move_sorter::MoveSorter,
    time_management::TimeManagement,
    transposition_table::TranspositionTable,
};

// Let's just use 1 billion instead of i32::MAX since I'm scared of overflow and underflow.
pub const INFINITY: i32 = 1_000_000_000;

pub static REDUCTIONS: LazyLock<[[u8; 32]; 32]> = LazyLock::new(|| {
    let mut reductions = [[0; 32]; 32];

    for depth in 1..32 {
        for moves in 1..32 {
            let reduction = 1.0 + (depth as f64).log(3.0) * (moves as f64).log(3.0) / 2.0;

            reductions[depth][moves] = reduction as u8;
        }
    }

    reductions
});

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Default)]
pub enum Bound {
    #[default]
    None,
    Exact,
    Upper,
    Lower,
}

pub struct SearchInfo {
    pub depth: usize,
    pub seldepth: usize,

    pub time: usize,
    pub nodes: usize,
    pub nps: usize,

    pub evaluation: isize,
    pub _best_move: ChessMove,
    pub pv: Vec<ChessMove>,
}

impl SearchInfo {
    pub fn print(&self) {
        let score = if Eval::mate_score(self.evaluation as i32) {
            let moves_to_mate = Eval::MATE_SCORE - self.evaluation.abs() as i32;
            let mate_in_moves = (moves_to_mate + 1) / 2;

            Score {
                mate: Some(self.evaluation.signum() * mate_in_moves as isize),
                ..Default::default()
            }
        } else {
            Score {
                cp: Some(self.evaluation),
                ..Default::default()
            }
        };

        let pv = self
            .pv
            .iter()
            .map(|mv| mv.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        let info = Info {
            depth: Some(self.depth),
            seldepth: Some(self.seldepth),
            pv: Some(pv),
            score: Some(score),
            time: Some(self.time),
            nodes: Some(self.nodes),
            nps: Some(self.nps),
            ..Default::default()
        };

        println!("{}", info);
    }
}

pub struct Search {
    board: Board,
    search_depth: u8,

    repetition_table: HashSet<u64>,
    transposition_table: Arc<TranspositionTable>,
    move_sorter: Arc<Mutex<MoveSorter>>,

    evaluation: i32,
    pv: Vec<ChessMove>,

    evaluation_iteration: i32,
    pv_iteration: Vec<ChessMove>,

    pub nodes: usize,
    pub seldepth: u8,

    pub think_timer: Instant,
    pub time_management: TimeManagement,

    pub cancelled: Arc<AtomicBool>,
}

impl Search {
    pub const MAX_PLY: u8 = 255;

    pub fn new(
        board: Board,
        depth: Option<u8>,
        time_management: TimeManagement,
        repetition_table: HashSet<u64>,
        transposition_table: Arc<TranspositionTable>,
        move_sorter: Arc<Mutex<MoveSorter>>,
        cancelled: Arc<AtomicBool>,
    ) -> Search {
        Search {
            board,
            search_depth: depth.unwrap_or(Search::MAX_PLY),

            repetition_table,
            transposition_table,
            move_sorter,

            evaluation: 1234567890,
            pv: Vec::new(),

            evaluation_iteration: 1234567890,
            pv_iteration: Vec::new(),

            nodes: 0,
            seldepth: 0,

            think_timer: Instant::now(),
            time_management,

            cancelled,
        }
    }

    pub fn start_search(&mut self) {
        let mut evaluation = 0;

        self.think_timer = Instant::now();
        for depth in 1..=self.search_depth {
            let mut delta = 16;

            let (mut alpha, mut beta) = if depth >= 5 {
                (evaluation - delta, evaluation + delta)
            } else {
                (-INFINITY, INFINITY)
            };

            loop {
                if self.should_cancel_search() {
                    break;
                }

                self.evaluation_iteration = self.search_base(alpha, beta, depth);

                evaluation = self.evaluation_iteration;

                if evaluation <= alpha {
                    alpha = alpha.saturating_sub(delta);
                    delta += delta / 3;

                    continue;
                }
                if evaluation >= beta {
                    beta = beta.saturating_add(delta);
                    delta += delta / 3;

                    continue;
                }

                if self.pv_iteration[0] != ChessMove::NULL_MOVE {
                    self.pv = self.pv_iteration.clone();
                    self.evaluation = self.evaluation_iteration;
                }

                break;
            }

            let elapsed = self.think_timer.elapsed().as_millis() as usize;

            let search_info = SearchInfo {
                depth: depth as usize,
                seldepth: self.seldepth as usize,
                time: elapsed,
                nodes: self.nodes,
                nps: (self.nodes as f32 * 1000.0 / elapsed.max(1) as f32).round() as usize,
                evaluation: self.evaluation as isize,
                _best_move: self.pv[0],
                pv: self.pv.clone(),
            };

            search_info.print();

            if self.should_cancel_search() {
                break;
            }
        }

        println!("bestmove {}", self.pv[0]);
    }

    pub fn should_cancel_search(&mut self) -> bool {
        self.time_management
            .should_cancel_search(self.think_timer, self.cancelled.clone())
    }

    pub fn search_base(&mut self, mut alpha: i32, beta: i32, depth: u8) -> i32 {
        let original_alpha = alpha;
        let mut legal_moves: u8 = 0;
        let mut max = i32::MIN;
        let mut best_move = ChessMove::NULL_MOVE;

        let zobrist_hash = self.board.hash();

        let inserted = if !self.repetition_table.contains(&zobrist_hash) {
            self.repetition_table.insert(zobrist_hash)
        } else {
            false
        };

        self.move_sorter.lock().unwrap().age_history();

        let first_move = self
            .transposition_table
            .probe(self.board.hash())
            .map_or(ChessMove::NULL_MOVE, |entry| entry.mv);

        let mut moves = self.board.generate_moves_vec(!EMPTY);
        self.move_sorter.lock().unwrap().sort_moves(&self.board, &mut moves, first_move, 1);
        for mv in moves {
            if let Ok(node_board) = self.board.make_move_new(mv) {
                let mut base_pv = [ChessMove::NULL_MOVE; 16];

                legal_moves += 1;
                let score = -self.search(&node_board, -beta, -alpha, depth - 1 + node_board.in_check() as u8, 1, &mut base_pv);

                if self.should_cancel_search() {
                    if best_move != ChessMove::NULL_MOVE {
                        break;
                    } else {
                        self.pv_iteration.clear();
                        self.pv_iteration.push(ChessMove::NULL_MOVE);

                        if inserted { self.repetition_table.remove(&zobrist_hash); }
                        return 0;
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
                if score >= beta {
                    self.transposition_table.store(
                        zobrist_hash,
                        depth,
                        Self::correct_store_mate_score(score, 0),
                        best_move,
                        Bound::Lower,
                    );
                    if inserted { self.repetition_table.remove(&zobrist_hash); }

                    return score;
                }
            }
        }

        if inserted {
            self.repetition_table.remove(&zobrist_hash);
        }

        if legal_moves == 0 {
            if self.board.in_check() {
                return -Eval::MATE_SCORE;
            } else {
                return 0;
            }
        }

        if max <= original_alpha {
            self.transposition_table.store(
                zobrist_hash,
                depth,
                Self::correct_store_mate_score(max, 0),
                best_move,
                Bound::Upper,
            );
        } else {
            self.transposition_table.store(
                zobrist_hash,
                depth,
                Self::correct_store_mate_score(max, 0),
                best_move,
                Bound::Exact,
            );
        }

        max
    }

    fn search(
        &mut self,
        board: &Board,
        mut alpha: i32,
        mut beta: i32,
        depth: u8,
        ply: u8,
        pv: &mut [ChessMove],
    ) -> i32 {
        if depth == 0 {
            return self.search_captures(board, alpha, beta, ply);
        }

        self.nodes += 1;

        let zobrist_hash = board.hash();

        if board.is_fifty_move() || self.repetition_table.contains(&zobrist_hash) {
            return 0;
        }

        let inserted = self.repetition_table.insert(zobrist_hash);

        let original_alpha = alpha;
        let mut legal_moves: u8 = 0;
        let mut max = i32::MIN;
        let mut best_move = ChessMove::NULL_MOVE;

        let is_pv = alpha != beta - 1;

        alpha = alpha.max(-Eval::MATE_SCORE + ply as i32);
        beta = beta.min(Eval::MATE_SCORE - ply as i32 - 1);

        if alpha >= beta {
            if inserted { self.repetition_table.remove(&zobrist_hash); }
            return alpha;
        }

        let entry = self.transposition_table.probe(zobrist_hash);

        let tt_mv = entry.map_or(ChessMove::NULL_MOVE, |entry| entry.mv);

        if let Some(entry) = entry
            && entry.depth >= depth
            && !is_pv
        {
            let corrected_score = Self::correct_probe_mate_score(entry.score, ply);

            match entry.bound {
                Bound::Exact => {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }
                    return corrected_score;
                }
                Bound::Lower if corrected_score >= beta => {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }

                    if !board.combined().is_set(tt_mv.to) {
                        self.move_sorter.lock().unwrap().update_history(
                            tt_mv.to,
                            unsafe { board.get_piece(tt_mv.from).unwrap_unchecked() },
                            depth as i16 * depth as i16,
                        );
                    }

                    return corrected_score;
                }
                Bound::Upper if corrected_score <= alpha => {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }
                    return corrected_score;
                }
                _ => {}
            }
        }

        if !is_pv
            && depth > 1
            && !board.in_check()
            && (board.occupancy(board.side_to_move)
                ^ board.pieces_color(Piece::Pawn, board.side_to_move))
            .count_ones()
                != 1
        {
            if let Ok(board) = board.make_null_move_new() {
                let mut node_pv = [ChessMove::NULL_MOVE; 16];

                let reduction = 3 + depth / 6;

                let mut score = -self.search(&board, -beta, -beta + 1, depth.saturating_sub(reduction), ply + 1, &mut node_pv);

                if score >= beta {
                    if Eval::mate_score(score) {
                        score = beta;
                    }

                    if inserted { self.repetition_table.remove(&zobrist_hash); }
                    return score;
                }
            }
        }

        let mut quiets = Vec::with_capacity(8);

        let mut moves = board.generate_moves_vec(!EMPTY);
        self.move_sorter.lock().unwrap().sort_moves(board, &mut moves, tt_mv, ply);
        for mv in moves {
            if let Ok(node_board) = board.make_move_new(mv) {
                let mut node_pv = [ChessMove::NULL_MOVE; 16];

                let is_quiet = !board.combined().is_set(mv.to);
                if is_quiet {
                    quiets.push(mv);
                }

                legal_moves += 1;

                let mut score = i32::MIN;

                // Don't reduce on captures, promotions and checks, because of instabilities.
                if depth >= 3
                    && legal_moves >= 3
                    && is_quiet
                    && !node_board.in_check()
                    && mv.promotion().is_none()
                {
                    // reduction quiet:   1 + log_3(depth) * log_3(legal_moves) * 1 / 2
                    // reduction capture: 0 + log_3(depth) * log_3(legal_moves) * 2 / 5

                    let reduction = REDUCTIONS[depth.min(31) as usize][legal_moves.min(31) as usize] - is_pv as u8;
                    let lmr_depth = (depth - 1).saturating_sub(reduction).max(1);

                    score = -self.search(&node_board, -alpha - 1, -alpha, lmr_depth, ply + 1, &mut node_pv);

                    if score > alpha {
                        score = -self.search(&node_board, -alpha - 1, -alpha, depth - 1, ply + 1, &mut node_pv);
                    }
                } else if !is_pv || legal_moves > 1 {
                    score = -self.search(&node_board, -alpha - 1, -alpha, depth - 1 + node_board.in_check() as u8, ply + 1, &mut node_pv);
                }

                if is_pv && (legal_moves == 1 || score > alpha) {
                    score = -self.search(&node_board, -beta, -alpha, depth - 1 + node_board.in_check() as u8, ply + 1, &mut node_pv);
                }

                if score > max {
                    max = score;
                    best_move = mv;

                    if score > alpha {
                        alpha = score;

                        pv[0] = mv;
                        pv[1..].copy_from_slice(&node_pv[..15]);
                    }
                }
                if score >= beta {
                    self.transposition_table.store(
                        zobrist_hash,
                        depth,
                        Self::correct_store_mate_score(score, ply),
                        mv,
                        Bound::Lower,
                    );
                    if inserted { self.repetition_table.remove(&zobrist_hash); }

                    if is_quiet {
                        self.move_sorter.lock().unwrap().update_history(
                            mv.to,
                            unsafe { board.get_piece(mv.from).unwrap_unchecked() },
                            depth as i16 * depth as i16,
                        );

                        quiets.pop();
                        for quiet in quiets {
                            self.move_sorter.lock().unwrap().update_history(
                                quiet.to,
                                unsafe { board.get_piece(quiet.from).unwrap_unchecked() },
                                -2 * depth as i16,
                            );
                        }

                        self.move_sorter.lock().unwrap().add_killer_move(mv, ply);
                    }

                    return score;
                }

                if self.nodes & 1023 == 0 && self.should_cancel_search() {
                    if inserted { self.repetition_table.remove(&zobrist_hash); }

                    return max;
                }
            }
        }

        if inserted { self.repetition_table.remove(&zobrist_hash); }

        if legal_moves == 0 {
            if board.in_check() {
                return -Eval::MATE_SCORE + ply as i32;
            } else {
                return 0;
            }
        }

        if best_move != ChessMove::NULL_MOVE {
            if max <= original_alpha {
                self.transposition_table.store(
                    zobrist_hash,
                    depth,
                    Self::correct_store_mate_score(max, ply),
                    best_move,
                    Bound::Upper,
                );
            } else {
                self.transposition_table.store(
                    zobrist_hash,
                    depth,
                    Self::correct_store_mate_score(max, ply),
                    best_move,
                    Bound::Exact,
                );
            }
        }

        max
    }

    fn search_captures(&mut self, board: &Board, mut alpha: i32, beta: i32, ply: u8) -> i32 {
        self.seldepth = self.seldepth.max(ply);
        self.nodes += 1;

        let stand_pat = Eval::new(board).eval();
        if stand_pat >= beta {
            return stand_pat;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let mut max = stand_pat;

        const FUTILITY_MARGIN: i32 = 170;
        let futility_base = stand_pat + FUTILITY_MARGIN;

        let mut moves = board.generate_moves_vec(board.occupancy(!board.side_to_move));
        self.move_sorter.lock().unwrap().sort_moves(board, &mut moves, ChessMove::NULL_MOVE, ply);
        for mv in moves {
            if let Ok(node_board) = board.make_move_new(mv) {
                if let Some(captured) = board.get_piece(mv.to) {
                    let futility_score = futility_base + PIECE_VALUES_EG[captured.to_index()];

                    if futility_score <= alpha
                        && !node_board.in_check()
                        && mv.promotion().is_none()
                    {
                        max = max.max(futility_score);
                        continue;
                    }
                }

                let score = -self.search_captures(&node_board, -beta, -alpha, ply + 1);

                if score > max {
                    max = score;

                    if score > alpha {
                        alpha = score;
                    }
                }
                if score >= beta {
                    return score;
                }
            }
        }

        max
    }

    fn correct_store_mate_score(score: i32, ply: u8) -> i32 {
        if Eval::mate_score(score) {
            let sign = score.signum();
            return score + sign * ply as i32;
        }
        score
    }

    fn correct_probe_mate_score(score: i32, ply: u8) -> i32 {
        if Eval::mate_score(score) {
            let sign = score.signum();
            return score - sign * ply as i32;
        }
        score
    }
}
