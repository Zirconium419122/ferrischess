use chessframe::{bitboard::EMPTY, board::Board, chess_move::ChessMove, piece::Piece};

use crate::eval::Eval;

pub struct Search<'a> {
    board: &'a Board,
    search_depth: usize,
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

    pub fn new(board: &Board, depth: usize) -> Search {
        Search {
            board,
            search_depth: depth,
        }
    }

    pub fn start_search(&self) -> (i32, Option<ChessMove>) {
        self.search_base()
    }

    fn search_base(&self) -> (i32, Option<ChessMove>) {
        let mut max = i32::MIN;
        let mut best_move = None;

        let alpha = -1_000_000_000;
        let beta = 1_000_000_000;

        let mut moves = self.board.generate_moves_vec(!EMPTY);
        Self::sort_moves(self.board, &mut moves);
        for mv in moves {
            if let Ok(board) = self.board.make_move_new(&mv) {
                let score = -self.search(&board, alpha, beta, self.search_depth - 1);

                if score > max {
                    max = score;
                    best_move = Some(mv);
                }
            }
        }

        (max, best_move)
    }

    fn search(&self, board: &Board, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        if depth == 0 {
            return Eval::new(board).eval();
        }

        let mut legal_moves = false;
        let mut max = i32::MIN;

        let mut moves = board.generate_moves_vec(!EMPTY);
        Self::sort_moves(board, &mut moves);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                legal_moves = true;
                let score = -self.search(&board, -beta, -alpha, depth - 1);

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

        if !legal_moves {
            if board.in_check() {
                return -Eval::MATE_SCORE + self.search_depth as i32 - depth as i32;
            } else {
                return 0;
            }
        }

        max
    }

    fn sort_moves(board: &Board, moves: &mut [ChessMove]) {
        moves.sort_by_key(|mv| -Self::score_move(board, mv));
    }

    fn score_move(board: &Board, mv: &ChessMove) -> i32 {
        let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };

        if let Some(captured) = board.get_piece(mv.to) {
            return Self::get_mvv_lva(captured, moved) as i32;
        }

        0
    }

    fn get_mvv_lva(victim: Piece, attacker: Piece) -> i8 {
        unsafe {
            *Self::MVV_LVA
                .get_unchecked(victim.to_index())
                .get_unchecked(attacker.to_index())
        }
    }
}
