use chessframe::{bitboard::EMPTY, board::Board, chess_move::ChessMove};

use crate::eval::Eval;

pub struct Search<'a>(&'a Board, usize);

impl<'a> Search<'a> {
    pub fn new(board: &Board, depth: usize) -> Search {
        Search(board, depth)
    }

    pub fn start_search(&self) -> (i32, Option<ChessMove>) {
        self.search_base()
    }

    pub fn search_base(&self) -> (i32, Option<ChessMove>) {
        let mut max = i32::MIN;
        let mut best_move = None;

        let moves = self.0.generate_moves_vec(!EMPTY);
        for mv in moves {
            if let Ok(board) = self.0.make_move_new(&mv) {
                let score = -self.search(&board, self.1 - 1);

                if score > max {
                    max = score;
                    best_move = Some(mv);
                }
            }
        }

        (max, best_move)
    }

    pub fn search(&self, board: &Board, depth: usize) -> i32 {
        if depth == 0 {
            return Eval::new(board).eval();
        }

        let mut legal_moves = false;
        let mut max = i32::MIN;

        let moves = board.generate_moves_vec(!EMPTY);
        for mv in moves {
            if let Ok(board) = board.make_move_new(&mv) {
                legal_moves = true;
                let score = -self.search(&board, depth - 1);

                if score > max {
                    max = score;
                }
            }
        }

        if !legal_moves {
            if board.in_check() {
                return -Eval::MATE_SCORE + self.1 as i32 - depth as i32;
            } else {
                return 0;
            }
        }

        max
    }
}