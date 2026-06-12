use chessframe::{
    bitboard::BitBoard,
    board::Board,
    chess_move::ChessMove,
    color::Color,
    magic::{get_bishop_moves, get_king_moves, get_knight_moves, get_pawn_attacks, get_rook_moves},
    piece::{PIECES, Piece},
    square::Square,
};

use crate::eval::Eval;

#[allow(dead_code)]
pub const MVV_LVA: [i8; 36] = [
    15, 14, 13, 12, 11, 10, // victim Pawn,   attacker P, N, B, R, Q, K
    25, 24, 23, 22, 21, 20, // victim Knight, attacker P, N, B, R, Q, K
    35, 34, 33, 32, 31, 30, // victim Bishop, attacker P, N, B, R, Q, K
    45, 44, 43, 42, 41, 40, // victim Rook,   attacker P, N, B, R, Q, K
    55, 54, 53, 52, 51, 50, // victim Queen,  attacker P, N, B, R, Q, K
     0,  0,  0,  0,  0,  0, // victim King,   attacker P, N, B, R, Q, K
];

const KILLER_MOVE_COUNT: usize = 12;

pub struct MoveSorter {
    pub history: [[i16; 64]; 6],
    pub killer_moves: [ChessMove; KILLER_MOVE_COUNT],
}

impl MoveSorter {
    pub fn new() -> MoveSorter {
        MoveSorter {
            history: [[0; 64]; 6],
            killer_moves: [ChessMove::NULL_MOVE; KILLER_MOVE_COUNT],
        }
    }

    pub fn clear(&mut self) {
        self.history = [[0; 64]; 6];
        self.killer_moves = [ChessMove::NULL_MOVE; KILLER_MOVE_COUNT];
    }

    pub fn age_history(&mut self) {
        for piece in &mut self.history {
            for score in piece {
                *score /= 2;
            }
        }
    }

    #[inline]
    pub fn update_history(&mut self, to: Square, piece: Piece, value: i16) {
        let entry = &mut self.history[piece.to_index()][to.to_index()];
        *entry = (*entry + value).clamp(-20_000, 20_000)
    }

    #[inline]
    pub fn add_killer_move(&mut self, mv: ChessMove, ply: u8) {
        if ply < KILLER_MOVE_COUNT as u8 {
            self.killer_moves[ply as usize] = mv
        }
    }

    pub fn sort_moves(
        &mut self,
        board: &Board,
        moves: &mut [ChessMove],
        tt_move: Option<ChessMove>,
        ply: u8,
    ) {
        let mut scored: Vec<(i32, ChessMove)> = moves
            .iter()
            .map(|&mv| (self.score_move(board, mv, tt_move, ply), mv))
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
        ply: u8,
    ) -> i32 {
        if Some(mv) == tt_move {
            return 200_000;
        }

        if let Some(promotion) = mv.promotion() {
            return 60_000 + Eval::piece_value(promotion);
        }

        let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };
        if board.get_piece(mv.to).is_some() {
            let see = self.see(board, mv);

            if see >= 0 {
                return 50_000 + see;
            } else {
                return 30_000 + see;
            }
        }

        if ply < KILLER_MOVE_COUNT as u8 && self.killer_moves[ply as usize] == mv {
            return 40_000;
        }

        self.history[moved.to_index()][mv.to.to_index()] as i32
    }

    fn see(&self, board: &Board, mv: ChessMove) -> i32 {
        let target = mv.to;

        let victim = match board.get_piece(target) {
            Some(piece) => piece,
            None => return 0,
        };

        let mut occ = board.combined();
        let mut side = board.side_to_move;
        let mut gain = [0; 16];
        let mut depth = 0;

        gain[0] = Eval::piece_value(victim);

        loop {
            let attackers = self.attackers_to(board, target, side, occ);
            if attackers.is_zero() {
                break;
            }

            let (from_square, attacker_piece) =
                self.least_valuable_attacker(board, side, attackers);
            depth += 1;

            gain[depth] = Eval::piece_value(attacker_piece) - gain[depth - 1];

            occ.clear_bit(from_square);

            side = !side;
        }

        while depth > 1 {
            depth -= 1;
            if gain[depth - 1] > -gain[depth] {
                gain[depth - 1] = -gain[depth]
            }
        }

        gain[0]
    }

    fn attackers_to(&self, board: &Board, square: Square, side: Color, occ: BitBoard) -> BitBoard {
        let mut attackers = BitBoard::default();

        attackers |= board.pieces_color(Piece::Pawn, side) & get_pawn_attacks(square, !side);
        attackers |= board.pieces_color(Piece::Knight, side) & get_knight_moves(square);
        attackers |= board.pieces_color(Piece::Bishop, side) & get_bishop_moves(square, occ);
        attackers |= board.pieces_color(Piece::Rook, side) & get_rook_moves(square, occ);
        attackers |= board.pieces_color(Piece::Queen, side) & (get_bishop_moves(square, occ) | get_rook_moves(square, occ));
        attackers |= board.pieces_color(Piece::King, side) & get_king_moves(square);

        attackers & occ
    }

    fn least_valuable_attacker(
        &self,
        board: &Board,
        side: Color,
        attackers: BitBoard,
    ) -> (Square, Piece) {
        for piece in PIECES {
            let bitboard = board.pieces_color(piece, side) & attackers;

            if bitboard.is_not_zero() {
                let square = bitboard.to_square();
                return (square, piece);
            }
        }

        unreachable!("attackers bitboard was empty")
    }

    #[inline]
    #[allow(dead_code)]
    fn get_mvv_lva(victim: Piece, attacker: Piece) -> i8 {
        unsafe { *MVV_LVA.get_unchecked(victim.to_index() * 6 + attacker.to_index()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn see_is_positive_for_safe_capture() {
        let fen = "7k/4r3/8/8/8/8/4Q3/4K3 w - - 0 1";
        let board = Board::from_fen(fen);

        let sorter = MoveSorter::new();
        let mv = ChessMove::new(Square::E2, Square::E7);

        assert!(
            sorter.see(&board, mv) > 0,
            "expected SEE to be positive for a free rook capture"
        );
    }

    #[test]
    fn see_is_negative_for_losing_capture() {
        let fen = "7k/8/3p4/4p3/8/5N2/8/4K3 w - - 0 1";
        let board = Board::from_fen(fen);

        let sorter = MoveSorter::new();
        let mv = ChessMove::new(Square::F3, Square::E5);

        assert!(
            sorter.see(&board, mv) < 0,
            "expected SEE to be negative for a losing knight capture"
        );
    }

    #[test]
    fn see_is_neutral_for_equal_capture() {
        let fen = "7k/8/2p5/3p4/4P3/8/8/4K3 w - - 0 1";
        let board = Board::from_fen(fen);

        let sorter = MoveSorter::new();
        let mv = ChessMove::new(Square::E4, Square::E5);

        assert!(
            sorter.see(&board, mv) == 0,
            "expected SEE to be neutral for a equal pawn capture"
        );
    }
}
