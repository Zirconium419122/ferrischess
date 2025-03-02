use chessframe::{board::Board, color::Color, piece::PIECES};

pub const PIECE_VALUES: [i32; 6] = [100, 300, 325, 500, 900, 0];

pub struct Eval<'a>(&'a Board);

impl<'a> Eval<'a> {
    pub const MATE_SCORE: i32 = 1_000_000_000;

    pub fn new(board: &Board) -> Eval {
        Eval(board)
    }

    pub fn eval(&self) -> i32 {
        let mut score = 0;

        for piece in PIECES.iter() {
            score += self.0.pieces_color(*piece, Color::White).count_ones() as i32
                * PIECE_VALUES[piece.to_index()];
            score -= self.0.pieces_color(*piece, Color::Black).count_ones() as i32
                * PIECE_VALUES[piece.to_index()];
        }

        let perspective = if self.0.side_to_move == Color::White {
            1
        } else {
            -1
        };
        score * perspective
    }
}
