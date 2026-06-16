use chessframe::{
    bitboard::{BitBoard, EMPTY},
    board::Board,
    color::Color,
    file::File,
    magic::{FILES, get_adjacent_files, get_bishop_moves, get_knight_moves, get_rook_moves},
    piece::{PIECES, Piece}, square::Square,
};

use crate::piecesquaretable::PieceSquareTable;

pub const PIECE_VALUES: [i32; 6] = [100, 310, 350, 500, 900, 0];

pub struct Eval<'a> {
    board: &'a Board,
}

impl Eval<'_> {
    pub const MATE_SCORE: i32 = 100_000_000;

    pub fn new(board: &Board) -> Eval<'_> {
        Eval { board }
    }

    pub fn piece_value(piece: Piece) -> i32 {
        let mg_score = unsafe { *PIECE_VALUES.get_unchecked(piece.to_index()) };
        let eg_score = unsafe { *PIECE_VALUES.get_unchecked(piece.to_index()) };

        mg_score + (eg_score << 16)
    }

    pub fn eval(&self) -> i32 {
        let mut score = 0;
        let mut mobility_score = 0;

        let game_phase = Self::calculate_game_phase(self.board);

        let mg_score = |score: i32| score as u16 as i16;
        let eg_score = |score: i32| (((score + 0x8000) as u32) >> 16) as u16 as i16;

        for piece in PIECES {
            for square in self.board.pieces_color(piece, Color::White) {
                score += Self::piece_value(piece)
                    + PieceSquareTable::read(square, piece, Color::White);

                mobility_score += self.mobility_score(square, piece, Color::White);
            }

            for square in self.board.pieces_color(piece, Color::Black) {
                score -= Self::piece_value(piece)
                    + PieceSquareTable::read(square, piece, Color::Black);

                mobility_score -= self.mobility_score(square, piece, Color::Black);
            }
        }

        score += self.pawn_structure_score(Color::White);
        score += self.pawn_structure_score(Color::Black);

        score += self.piece_combination_score(Color::White);
        score += self.piece_combination_score(Color::Black);

        score += mobility_score;

        score = (mg_score(score) as i32 * (256 - game_phase) + eg_score(score) as i32 * game_phase) / 256;

        if self.board.in_check() {
            score -= 50;
        }

        if self.board.side_to_move == Color::White {
            score
        } else {
            -score
        }
    }

    pub fn pawn_structure_score(&self, color: Color) -> i32 {
        let pawns = self.board.pieces_color(Piece::Pawn, color);
        let mut score = 0;

        for (i, file) in FILES.iter().enumerate() {
            let on_file = (pawns & file).count_ones() as i32;

            if on_file > 1 {
                let doubles = on_file - 1;
                score -= doubles * 25 + ((doubles * 25) << 16);
            }

            if pawns & get_adjacent_files(File::from_index(i)) == EMPTY {
                score -= 25 + (25 << 16);
            }
        }

        if color == Color::White {
            score
        } else {
            -score
        }
    }

    pub fn piece_combination_score(&self, color: Color) -> i32 {
        let mut score = 0;

        if self.board.pieces_color(Piece::Bishop, color).count_ones() >= 2 {
            score += 30 + (80 << 16);
        }

        if self.board.pieces_color(Piece::Knight, color).count_ones() >= 2 {
            score += 5 + (-10 << 16);
        }

        if color == Color::White {
            score
        } else {
            -score
        }
    }

    pub fn mobility_score(&self, square: Square, piece: Piece, color: Color) -> i32 {
        let allied_pieces = self.board.occupancy(color);
        let combined = self.board.combined();

        let pawn_attacks = self.pawn_attacks(!color);

        let mobility = (match piece {
            Piece::Knight => get_knight_moves(square),
            Piece::Bishop => get_bishop_moves(square, combined),
            Piece::Rook => get_rook_moves(square, combined),
            Piece::Queen => get_bishop_moves(square, combined) | get_rook_moves(square, combined),
            _ => return 0,
        } & !allied_pieces & !pawn_attacks).count_ones() as i32;

        match piece {
            Piece::Knight => mobility * 4 + ((mobility * 4) << 16),
            Piece::Bishop => mobility * 5 + ((mobility * 5) << 16),
            Piece::Rook => mobility * 2 + ((mobility * 4) << 16),
            Piece::Queen => mobility + ((mobility * 2) << 16),
            _ => unreachable!(),
        }
    }

    pub fn calculate_game_phase(board: &Board) -> i32 {
        const TOTAL_PHASE: u32 = 24;

        let mut phase = 0;

        phase += board.pieces(Piece::Knight).count_ones();
        phase += board.pieces(Piece::Bishop).count_ones();
        phase += 2 * board.pieces(Piece::Rook).count_ones();
        phase += 4 * board.pieces(Piece::Queen).count_ones();

        let clamped = phase.min(TOTAL_PHASE);
        256 * (24 - clamped as i32) / 24
    }

    pub fn mate_score(score: i32) -> bool {
        score.abs() >= Eval::MATE_SCORE - 1000
    }

    fn pawn_attacks(&self, color: Color) -> BitBoard {
        match color {
            Color::White => {
                ((self.board.pieces_color(Piece::Pawn, color) << 7) & !BitBoard(0x8080808080808080))
                    | ((self.board.pieces_color(Piece::Pawn, color) << 9)
                        & !BitBoard(0x1010101010101010))
            }
            Color::Black => {
                ((self.board.pieces_color(Piece::Pawn, color) >> 7) & !BitBoard(0x1010101010101010))
                    | ((self.board.pieces_color(Piece::Pawn, color) >> 9)
                        & !BitBoard(0x8080808080808080))
            }
        }
    }
}
