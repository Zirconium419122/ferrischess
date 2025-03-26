use std::collections::HashSet;

use chessframe::{
    bitboard::{BitBoard, EMPTY},
    board::Board,
    chess_move::ChessMove,
    color::Color,
    piece::Piece,
    transpositiontable::TranspositionTable,
};

use crate::eval::Eval;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Default)]
pub enum Bound {
    #[default]
    None,
    Exact,
    Upper,
    Lower,
}

pub struct Search<'a> {
    board: &'a Board,
    search_depth: usize,

    repetition_table: HashSet<u64>,
    transposition_table: &'a mut TranspositionTable<(i32, Bound, ChessMove)>,

    best_move: Option<ChessMove>,
    evaluation: i32,
    pv: Vec<ChessMove>,

    pub nodes: usize,
}

impl<'a> Search<'a> {
    const MVV_LVA: [[i8; 6]; 6] = [
        [15, 14, 13, 12, 11, 10], // victim Pawn, attacker P, N, B, R, Q, K
        [25, 24, 23, 22, 21, 20], // victim Knight, attacker P, N, B, R, Q, K
        [35, 34, 33, 32, 31, 30], // victim Bishop, attacker P, N, B, R, Q, K
        [45, 44, 43, 42, 41, 40], // victim Rook, attacker P, N, B, R, Q, K
        [55, 54, 53, 52, 51, 50], // victim Queen, attacker P, N, B, R, Q, K
        [0, 0, 0, 0, 0, 0],       // victim King, attacker P, N, B, R, Q, K
    ];

    pub fn new(
        board: &'a Board,
        depth: usize,
        repetition_table: HashSet<u64>,
        transposition_table: &'a mut TranspositionTable<(i32, Bound, ChessMove)>,
    ) -> Search<'a> {
        Search {
            board,
            search_depth: depth,
            repetition_table,
            transposition_table,
            best_move: None,
            evaluation: 1234567890,
            pv: Vec::new(),
            nodes: 0,
        }
    }

    pub fn start_search(&mut self) -> (i32, Option<ChessMove>, Vec<ChessMove>) {
        let search_depth = self.search_depth;

        for i in 1..=search_depth {
            self.search_depth = i;

            (self.evaluation, self.best_move) = self.search_base();
        }

        (self.evaluation, self.best_move, self.pv.clone())
    }

    pub fn search_base(&mut self) -> (i32, Option<ChessMove>) {
        let mut legal_moves = false;
        let mut max = i32::MIN;
        let mut best_move = None;

        let mut inserted = false;
        let zobrist_hash = self.board.hash();
        if !self.repetition_table.contains(&zobrist_hash) {
            inserted = true;
            self.repetition_table.insert(zobrist_hash);
        }

        let alpha = -1_000_000_000;
        let beta = 1_000_000_000;

        let first_move = self
            .transposition_table
            .get(self.board.hash())
            .map(|entry| entry.value.2);

        let mut moves = self.board.generate_moves_vec(!EMPTY);
        Self::sort_moves(self.board, &mut moves, first_move);
        for mv in moves {
            if let Ok(board) = self.board.make_move_new(&mv) {
                let mut base_pv = Vec::new();

                legal_moves = true;
                let score = -self.search(&board, alpha, beta, self.search_depth - 1, &mut base_pv);

                if score > max {
                    max = score;
                    best_move = Some(mv);

                    self.pv.clear();
                    self.pv.push(mv);
                    self.pv.append(&mut base_pv);
                }
            }
        }

        if inserted {
            let _ = self.repetition_table.remove(&zobrist_hash);
        }

        if !legal_moves {
            if self.board.in_check() {
                return (-Eval::MATE_SCORE, None);
            } else {
                return (0, None);
            }
        }

        (max, best_move)
    }

    fn search(&mut self, board: &Board, mut alpha: i32, beta: i32, mut depth: usize, pv: &mut Vec<ChessMove>) -> i32 {
        if board.in_check() {
            depth += 1;
        }

        if depth == 0 {
            return self.search_captures(board, alpha, beta);
        }

        self.nodes += 1;

        let inserted;
        let zobrist_hash = board.hash();
        if !self.repetition_table.contains(&zobrist_hash) {
            inserted = true;
            self.repetition_table.insert(zobrist_hash);
        } else {
            return 0;
        }

        let original_alpha = alpha;
        let mut legal_moves = false;
        let mut max = i32::MIN;
        let mut best_move = None;

        let entry = self.transposition_table.get(board.hash());

        if let Some(entry) = entry {
            if entry.depth >= depth as u8 {
                let corrected_score =
                    Self::correct_mate_score(entry.value.0, self.search_depth - depth);
                match entry.value.1 {
                    Bound::Exact => return corrected_score,
                    // Bound::Lower if corrected_score >= beta => return corrected_score,
                    Bound::Upper if corrected_score <= alpha => return corrected_score,
                    _ => {}
                }
            }
        }

        let mut moves = board.generate_moves_vec(!EMPTY);
        Self::sort_moves(board, &mut moves, entry.map(|entry| entry.value.2));
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                let mut node_pv = Vec::new();

                legal_moves = true;
                let score = -self.search(&board, -beta, -alpha, depth.saturating_sub(1), &mut node_pv);

                if score > max {
                    max = score;
                    best_move = Some(mv);
                }
                if score > alpha {
                    alpha = score;

                    pv.clear();
                    pv.push(mv);
                    pv.append(&mut node_pv);
                }
                if score >= beta {
                    self.transposition_table.store(
                        board.hash(),
                        (beta, Bound::Lower, best_move.unwrap_or(mv)),
                        depth as u8,
                    );
                    if inserted {
                        let _ = self.repetition_table.remove(&zobrist_hash);
                    }
                    return beta;
                }
            }
        }

        if inserted {
            let _ = self.repetition_table.remove(&zobrist_hash);
        }

        if !legal_moves {
            if board.in_check() {
                return -Eval::MATE_SCORE + self.search_depth as i32 - depth as i32;
            } else {
                return 0;
            }
        }

        if let Some(best_move) = best_move {
            if beta <= alpha && alpha <= original_alpha {
                self.transposition_table.store(
                    board.hash(),
                    (alpha, Bound::Exact, best_move),
                    depth as u8,
                );
            } else if alpha <= original_alpha {
                self.transposition_table.store(
                    board.hash(),
                    (alpha, Bound::Upper, best_move),
                    depth as u8,
                );
            };
        }

        alpha
    }

    fn search_captures(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        const EVAL_MARGIN: i32 = 25;

        let eval = Eval::new(board).eval();
        if eval + EVAL_MARGIN >= beta {
            self.nodes += 1;
            return eval;
        }
        if eval > alpha {
            alpha = eval;
        }

        self.nodes += 1;

        let mut moves = board.generate_moves_vec(board.occupancy(!board.side_to_move));
        Self::sort_moves(board, &mut moves, None);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                let score = -self.search_captures(&board, -beta, -alpha);

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

    fn sort_moves(board: &Board, moves: &mut [ChessMove], first_move: Option<ChessMove>) {
        let pawn_attack_mask = Self::pawn_attack_mask(board, !board.side_to_move);
        if let Some(first_move) = first_move {
            moves.sort_by_key(|mv| {
                if mv == &first_move {
                    -1000
                } else {
                    -Self::score_move(board, pawn_attack_mask, mv)
                }
            });
        } else {
            moves.sort_by_key(|mv| -Self::score_move(board, pawn_attack_mask, mv));
        }
    }

    fn score_move(board: &Board, pawn_attack_mask: BitBoard, mv: &ChessMove) -> i32 {
        let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };

        let mut score = 0;

        if pawn_attack_mask & BitBoard::from_square(mv.to) != EMPTY {
            score -= 20;
        }

        if let Some(captured) = board.get_piece(mv.to) {
            score += Self::get_mvv_lva(captured, moved) as i32;
        }

        score
    }

    fn get_mvv_lva(victim: Piece, attacker: Piece) -> i8 {
        unsafe {
            *Self::MVV_LVA
                .get_unchecked(victim.to_index())
                .get_unchecked(attacker.to_index())
        }
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

    fn correct_mate_score(score: i32, ply: usize) -> i32 {
        if score.abs() > Eval::MATE_SCORE - 1000 {
            let sign = score.signum();
            return (score * sign - ply as i32) * sign;
        }
        score
    }
}
