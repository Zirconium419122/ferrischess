use chessframe::{
    bitboard::EMPTY,
    board::Board,
    color::{COLORS, Color},
    file::File,
    magic::{FILES, get_adjacent_files},
    piece::Piece,
};

use crate::piecesquaretable::PieceSquareTable;

pub const PIECE_VALUES: [i32; 6] = [100, 310, 325, 500, 900, 0];

pub struct Eval<'a> {
    board: &'a Board,
}

impl Eval<'_> {
    pub const MATE_SCORE: i32 = 100_000_000;

    pub fn new(board: &Board) -> Eval<'_> {
        Eval { board }
    }

    pub fn piece_value(piece: Piece) -> i32 {
        unsafe { *PIECE_VALUES.get_unchecked(piece.to_index()) }
    }

    pub fn eval(&self) -> i32 {
        let mut score = 0;

        let game_phase = self.calculate_game_phase();

        for square in self.board.occupancy(Color::White) {
            let piece = unsafe { self.board.get_piece(square).unwrap_unchecked() };
            score += Self::piece_value(piece)
                + PieceSquareTable::read(square, piece, Color::White, game_phase) as i32;
        }

        for square in self.board.occupancy(Color::Black) {
            let piece = unsafe { self.board.get_piece(square).unwrap_unchecked() };
            score -= Self::piece_value(piece)
                + PieceSquareTable::read(square, piece, Color::Black, game_phase) as i32;
        }

        for color in COLORS {
            let pawns = self.board.pieces_color(Piece::Pawn, color);
            let mut penalty = 0;

            for (i, file) in FILES.iter().enumerate() {
                penalty += (pawns & file).count_ones().saturating_sub(1) as i32;
                penalty += (pawns & get_adjacent_files(File::from_index(i)) == EMPTY) as i32;
            }

            if color == Color::White {
                penalty *= 25;
            } else {
                penalty *= -25;
            }

            score -= penalty;
        }

        if self.board.in_check() {
            score -= 50;
        }

        if self.board.side_to_move == Color::White {
            score
        } else {
            score.wrapping_neg()
        }
    }

    fn calculate_game_phase(&self) -> f32 {
        const TOTAL_PHASE: u32 = 24;

        let mut phase = 0;

        phase += self.board.pieces(Piece::Knight).count_ones();
        phase += self.board.pieces(Piece::Bishop).count_ones();
        phase += 2 * self.board.pieces(Piece::Rook).count_ones();
        phase += 4 * self.board.pieces(Piece::Queen).count_ones();

        let clamped = phase.min(TOTAL_PHASE);
        1.0 - (clamped as f32 / TOTAL_PHASE as f32)
    }

    pub fn mate_score(score: i32) -> bool {
        score.abs() >= Eval::MATE_SCORE - 1000
    }
}
