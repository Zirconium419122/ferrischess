use chessframe::{color::Color, piece::Piece, rank::Rank, square::Square};

pub struct PieceSquareTable;

#[rustfmt::skip]
impl PieceSquareTable {
    pub const PAWN: [i8; 64] = [
          0,  0,  0,  0,  0,  0,  0,  0,
         60, 60, 60, 60, 60, 60, 60, 60,
         20, 20, 20, 25, 25, 20, 20, 20,
         10, 10, 10, 25, 25,  0,  0,  0,
          0,  0,  0, 20, 20,  0,  0,  0,
         15,  0,  0,  0,  0,  0,  0, 15,
          0, 10, 10,-20,-20, 10, 10,  0,
          0,  0,  0,  0,  0,  0,  0,  0,
    ];

    pub const PAWN_END: [i8; 64] = [
          0,  0,  0,  0,  0,  0,  0,  0,
         80, 80, 80, 80, 80, 80, 80, 80,
         50, 50, 50, 50, 50, 50, 50, 50,
         30, 30, 30, 30, 30, 30, 30, 30,
         20, 20, 20, 20, 20, 20, 20, 20,
         10, 10, 10, 10, 10, 10, 10, 10,
         10, 10, 10, 10, 10, 10, 10, 10,
          0,  0,  0,  0,  0,  0,  0,  0,
    ];

    pub const KNIGHT: [i8; 64] = [
        -50,-40,-30,-30,-30,-30,-40,-50,
        -40,-20,  0,  0,  0,  0,-20,-40,
        -30,  0, 10, 15, 15, 10,  0,-30,
        -30,  0, 15, 20, 20, 15,  0,-30,
        -30,  0, 15, 20, 20, 15,  0,-30,
        -30,  0, 10, 15, 15, 10,  0,-30,
        -40,-20,  0,  5,  5,  0,-20,-40,
        -50,-40,-30,-30,-30,-30,-40,-50,
    ];

    pub const BISHOP: [i8; 64] = [
        -20,-10,-10,-10,-10,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5, 10, 10,  5,  0,-10,
        -10,  5,  5, 10, 10,  5,  5,-10,
        -10,  0, 10, 10, 10, 10,  0,-10,
        -10, 10, 10, 10, 10, 10, 10,-10,
        -10,  5,  0,  0,  0,  0,  5,-10,
        -20,-10,-10,-10,-10,-10,-10,-20,
    ];

    pub const ROOK: [i8; 64] = [
         0,  0,  0,  0,  0,  0,  0,  0,
         5, 10, 10, 10, 10, 10, 10,  5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
         0,  0,  0,  5,  5,  0,  0,  0,
    ];

    pub const QUEEN: [i8; 64] = [
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
         -5,  0,  5,  5,  5,  5,  0, -5,
        -10,  0,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20,
    ];

    pub const KING: [i8; 64] = [
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -20,-30,-30,-40,-40,-30,-30,-20,
        -10,-20,-20,-20,-20,-20,-20,-10,
         30, 10,  0,  0,  0,  0, 10, 30,
         25, 30, 10,  0,  0, 10, 30, 25,
    ];

    pub const KING_END: [i8; 64] = [
        -50,-40,-30,-20,-20,-30,-40,-50,
        -30,-20,-10,  0,  0,-10,-20,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-30,  0,  0,  0,  0,-30,-30,
        -50,-30,-30,-30,-30,-30,-30,-50,
    ];

    pub const TABLES: [[i8; 64]; 6] = [
        Self::PAWN,
        Self::KNIGHT,
        Self::BISHOP,
        Self::ROOK,
        Self::QUEEN,
        Self::KING,
    ];

    pub fn read(square: Square, piece: Piece, color: Color, game_phase: f32) -> i8 {
        let mut square = square;

        if color == Color::White {
            let file = square.file();
            let rank = Rank::from_index(7 - square.rank().to_index());

            square = Square::make_square(rank, file);
        }

        if piece == Piece::Pawn {
            let pawn_start = unsafe {
                Self::PAWN.get_unchecked(square.to_index())
            };
            let pawn_end = unsafe {
                Self::PAWN_END.get_unchecked(square.to_index())
            };
            let interpolated = ((*pawn_start as f32 * (1.0 - game_phase)) + (*pawn_end as f32 * game_phase)) as i8;

            return interpolated;
        } else if piece == Piece::King {
            let king_start = unsafe {
                Self::KING.get_unchecked(square.to_index())
            };
            let king_end = unsafe {
                Self::KING_END.get_unchecked(square.to_index())
            };
            let interpolated = ((*king_start as f32 * (1.0 - game_phase)) + (*king_end as f32 * game_phase)) as i8;

            return interpolated;
        }

        unsafe {
            *Self::TABLES
                .get_unchecked(piece.to_index())
                .get_unchecked(square.to_index())
        }
    }
}
