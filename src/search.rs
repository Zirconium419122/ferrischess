use chessframe::{
    bitboard::{BitBoard, EMPTY},
    board::Board,
    chess_move::ChessMove,
    color::Color,
    piece::Piece,
};

use crate::eval::Eval;

pub struct Search<'a> {
    board: &'a Board,
    search_depth: usize,
    repetition_table: Vec<u64>,
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

    pub fn new(board: &Board, depth: usize, repetition_table: Vec<u64>) -> Search {
        Search {
            board,
            search_depth: depth,
            repetition_table,
            nodes: 0,
        }
    }

    pub fn start_search(&mut self) -> (i32, Option<ChessMove>) {
        self.search_base()
    }

    fn search_base(&mut self) -> (i32, Option<ChessMove>) {
        let mut max = i32::MIN;
        let mut best_move = None;
        let mut legal_moves = false;

        let zobrist_hash = self.board.hash();
        self.repetition_table.push(zobrist_hash);

        let alpha = -1_000_000_000;
        let beta = 1_000_000_000;

        let mut moves = self.board.generate_moves_vec(!EMPTY);
        Self::sort_moves(self.board, &mut moves);
        for mv in moves {
            if let Ok(board) = self.board.make_move_new(&mv) {
                legal_moves = true;
                let score = -self.search(&board, alpha, beta, self.search_depth - 1);

                self.nodes += 1;

                if score > max {
                    max = score;
                    best_move = Some(mv);
                }
            }
        }

        if !legal_moves {
            let _ = self.repetition_table.pop();

            if self.board.in_check() {
                return (-Eval::MATE_SCORE, None);
            } else {
                return (0, None);
            }
        }

        let _ = self.repetition_table.pop();

        (max, best_move)
    }

    fn search(&mut self, board: &Board, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        if depth == 0 && !board.in_check() {
            let zobrist_hash = board.hash();
            if self.repetition_table.iter().any(|&x| x == zobrist_hash) {
                return 0;
            }

            return self.search_captures(board, alpha, beta);
        }

        let zobrist_hash = board.hash();
        if self.repetition_table.iter().any(|&x| x == zobrist_hash) {
            return 0;
        } else {
            self.repetition_table.push(zobrist_hash);
        }

        let mut legal_moves = false;
        let mut max = i32::MIN;

        let mut moves = board.generate_moves_vec(!EMPTY);
        Self::sort_moves(board, &mut moves);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                legal_moves = true;
                let score = -self.search(&board, -beta, -alpha, depth.saturating_sub(1));

                self.nodes += 1;

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

        let _ = self.repetition_table.pop();

        if !legal_moves {
            if board.in_check() {
                return -Eval::MATE_SCORE + self.search_depth as i32 - depth as i32;
            } else {
                return 0;
            }
        }

        max
    }

    fn search_captures(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        const EVAL_MARGIN: i32 = 25;

        let eval = Eval::new(board).eval();
        if eval + EVAL_MARGIN >= beta {
            return eval;
        }
        if eval > alpha {
            alpha = eval;
        }

        let mut moves = board.generate_moves_vec(board.occupancy(!board.side_to_move));
        Self::sort_moves(board, &mut moves);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                let score = -self.search_captures(&board, -beta, -alpha);

                self.nodes += 1;

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

    fn sort_moves(board: &Board, moves: &mut [ChessMove]) {
        let pawn_attack_mask = Self::pawn_attack_mask(board, !board.side_to_move);
        moves.sort_by_key(|mv| -Self::score_move(board, pawn_attack_mask, mv));
    }

    fn score_move(board: &Board, pawn_attack_mask: BitBoard, mv: &ChessMove) -> i32 {
        let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };

        let mut score = 0;

        if pawn_attack_mask & BitBoard::from_square(mv.to) != EMPTY {
            score -= 40;
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
}
