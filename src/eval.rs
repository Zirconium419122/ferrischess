use chessframe::{
    board::Board,
    color::Color,
    piece::{Piece, PIECES},
};

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

        for piece in PIECES.iter() {
            score += self.board.pieces_color(*piece, Color::White).count_ones() as i32
                * Self::piece_value(piece);
            score -= self.board.pieces_color(*piece, Color::Black).count_ones() as i32
                * Self::piece_value(piece);
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
