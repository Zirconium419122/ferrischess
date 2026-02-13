use chessframe::{board::Board, chess_move::ChessMove, piece::Piece};

use crate::search::Search;

pub const MVV_LVA: [i8; 36] = [
    15, 14, 13, 12, 11, 10, // victim Pawn, attacker P, N, B, R, Q, K
    25, 24, 23, 22, 21, 20, // victim Knight, attacker P, N, B, R, Q, K
    35, 34, 33, 32, 31, 30, // victim Bishop, attacker P, N, B, R, Q, K
    45, 44, 43, 42, 41, 40, // victim Rook, attacker P, N, B, R, Q, K
    55, 54, 53, 52, 51, 50, // victim Queen, attacker P, N, B, R, Q, K
     0,  0,  0,  0,  0,  0, // victim King, attacker P, N, B, R, Q, K
];

pub struct MoveSorter {
    pub killer_moves: [ChessMove; 16]
}

impl MoveSorter {
    pub fn new() -> MoveSorter {
        MoveSorter {
            killer_moves: [Search::NULL_MOVE; 16]
        }
    }

    pub fn add_killer_move(&mut self, mv: ChessMove, ply: u8) {
        if ply < 16 {
            self.killer_moves[ply as usize] = mv
        }
    }

    pub fn sort_moves(
        &mut self,
        board: &Board,
        moves: &mut [ChessMove],
        tt_move: Option<ChessMove>,
        pv_move: Option<ChessMove>,
        ply: u8,
    ) {
        let mut scored: Vec<(i32, ChessMove)> = moves
            .iter()
            .map(|&mv| (self.score_move(board, mv, tt_move, pv_move, ply), mv))
            .collect();

        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        for (i, (_, mv)) in scored.into_iter().enumerate() {
            moves[i] = mv;
        }
    }

    #[inline]
    fn score_move(
        &self,
        board: &Board,
        mv: ChessMove,
        tt_move: Option<ChessMove>,
        pv_move: Option<ChessMove>,
        ply: u8,
    ) -> i32 {
        if Some(mv) == pv_move {
            return 20000;
        }

        if Some(mv) == tt_move {
            return 10000;
        }

        if let Some(captured) = board.get_piece(mv.to) {
            let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };

            return 1000 + Self::get_mvv_lva(captured, moved) as i32;
        } else if ply < 16 && self.killer_moves[ply as usize] == mv {
            return 5000;
        }

        0
    }

    #[inline]
    fn get_mvv_lva(victim: Piece, attacker: Piece) -> i8 {
        unsafe {
            *MVV_LVA
                .get_unchecked(victim.to_index() * 6 + attacker.to_index())
        }
    }
}