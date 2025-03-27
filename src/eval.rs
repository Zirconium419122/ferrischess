use chessframe::{
    bitboard::EMPTY,
    board::Board,
    color::{Color, COLORS},
    file::File,
    magic::{get_adjacent_files, FILES},
    piece::Piece,
};

use crate::piecesquaretable::PieceSquareTable;

pub const PIECE_VALUES: [i32; 6] = [100, 310, 325, 500, 900, 0];

pub struct Eval<'a> {
    board: &'a Board,
}

impl<'a> Eval<'a> {
    pub const MATE_SCORE: i32 = 1_000_000_000;

    pub fn new(board: &Board) -> Eval {
        Eval { board }
    }

    pub fn piece_value(piece: &Piece) -> i32 {
        unsafe { *PIECE_VALUES.get_unchecked(piece.to_index()) }
    }

    pub fn eval(&self) -> i32 {
        let mut score = 0;

        for square in self.board.occupancy(Color::White) {
            let piece = unsafe { self.board.get_piece(square).unwrap_unchecked() };
            score += Self::piece_value(&piece)
                + PieceSquareTable::read(square, piece, Color::White) as i32;
        }

        for square in self.board.occupancy(Color::Black) {
            let piece = unsafe { self.board.get_piece(square).unwrap_unchecked() };
            score -= Self::piece_value(&piece)
                + PieceSquareTable::read(square, piece, Color::Black) as i32;
        }

        for color in COLORS {
            let pawns = self.board.pieces_color(Piece::Pawn, color);
            let mut penalty = 0;

            for (i, file) in FILES.iter().enumerate() {
                penalty += (pawns & file).count_ones().saturating_sub(1) as i32;
                penalty += if (pawns & get_adjacent_files(File::from_index(i))) == EMPTY {
                    1
                } else {
                    0
                };
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

        let perspective = if self.board.side_to_move == Color::White {
            1
        } else {
            -1
        };
        score * perspective
    }
}
