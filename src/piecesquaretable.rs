use chessframe::{color::Color, piece::Piece, rank::Rank, square::Square};

pub struct PieceSquareTable;

impl PieceSquareTable {
    pub const PAWN: [i8; 64] = [
          0,  0,  0,  0,  0,  0,  0,  0,
         60, 60, 60, 60, 60, 60, 60, 60,
         20, 20, 20, 20, 20, 20, 20, 20,
          0,  0,  0, 25, 25,  0,  0,  0,
          0,  0,  0, 20, 20,  0,  0,  0,
         15,  0,  0,  0,  0,  0,  0, 15,
          5, 10, 10,-20,-20, 10, 10,  5,
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

    pub const BISHOP: [i8; 64] = [0; 64];

    pub const ROOK: [i8; 64] = [0; 64];

    pub const QUEEN: [i8; 64] = [0; 64];

    pub const KING: [i8; 64] = [
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -20,-30,-30,-40,-40,-30,-30,-20,
        -10,-20,-20,-20,-20,-20,-20,-10,
         20, 10,  0,  0,  0,  0, 10, 20,
         20, 30, 10,  0,  0, 10, 30, 20,
    ];

    pub const TABLES: [[i8; 64]; 6] = [
        Self::PAWN,
        Self::KNIGHT,
        Self::BISHOP,
        Self::ROOK,
        Self::QUEEN,
        Self::KING,
    ];

    pub fn read(square: Square, piece: Piece, color: Color) -> i8 {
        let mut square = square;

        if color == Color::White {
            let file = square.file();
            let rank = Rank::from_index(7 - square.rank().to_index());

            square = Square::make_square(rank, file);
        }

        unsafe {
            *Self::TABLES
                .get_unchecked(piece.to_index())
                .get_unchecked(square.to_index())
        }
    }
}
