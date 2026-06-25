use chessframe::{
    bitboard::{BitBoard, EMPTY},
    board::Board,
    color::Color,
    file::File,
    magic::{FILES, get_adjacent_files, get_bishop_moves, get_knight_moves, get_rook_moves},
    piece::{PIECES, Piece},
    square::Square,
};

use crate::piecesquaretable::PieceSquareTable;

#[inline(always)]
pub const fn s(mg: i32, eg: i32) -> i32 {
    mg + (eg << 16)
}

pub const PIECE_VALUES_MG: [i32; 6] = [100, 310, 350, 500, 900, 0];
pub const PIECE_VALUES_EG: [i32; 6] = [100, 310, 350, 500, 900, 0];

pub struct Eval<'a> {
    board: &'a Board,
}

impl Eval<'_> {
    pub const MATE_SCORE: i32 = 100_000_000;

    pub fn new(board: &Board) -> Eval<'_> {
        Eval { board }
    }

    pub fn piece_value(piece: Piece) -> i32 {
        let mg_score = unsafe { *PIECE_VALUES_MG.get_unchecked(piece.to_index()) };
        let eg_score = unsafe { *PIECE_VALUES_EG.get_unchecked(piece.to_index()) };

        s(mg_score, eg_score)
    }

    pub fn eval(&self) -> i32 {
        let mut score = 0;
        let mut mobility_score = 0;

        let game_phase = Self::calculate_game_phase(self.board);

        if game_phase > 200 && self.insufficient_material() {
            return 0;
        }

        let mg_score = |score: i32| score as u16 as i16;
        let eg_score = |score: i32| (((score + 0x8000) as u32) >> 16) as u16 as i16;

        for piece in PIECES {
            for square in self.board.pieces_color(piece, Color::White) {
                score +=
                    Self::piece_value(piece) + PieceSquareTable::read(square, piece, Color::White);

                mobility_score += self.mobility_score(square, piece, Color::White);
            }

            for square in self.board.pieces_color(piece, Color::Black) {
                score -=
                    Self::piece_value(piece) + PieceSquareTable::read(square, piece, Color::Black);

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
                score -= s(doubles * 20, doubles * 40);
            }

            if pawns & get_adjacent_files(File::from_index(i)) == EMPTY {
                score -= s(10, 20);
            }
        }

        if color == Color::White { score } else { -score }
    }

    pub fn piece_combination_score(&self, color: Color) -> i32 {
        let mut score = 0;

        if self.board.pieces_color(Piece::Bishop, color).count_ones() >= 2 {
            score += s(30, 80);
        }

        if self.board.pieces_color(Piece::Knight, color).count_ones() >= 2 {
            score += s(5, -10);
        }

        if color == Color::White { score } else { -score }
    }

    pub fn insufficient_material(&self) -> bool {
        if self.board.pieces(Piece::Pawn).count_ones() != 0
            || self.board.pieces(Piece::Rook).count_ones() != 0
            || self.board.pieces(Piece::Queen).count_ones() != 0
        {
            return false;
        }

        let knight_count = self.board.pieces(Piece::Knight).count_ones();
        let bishop_count = self.board.pieces(Piece::Bishop).count_ones();
        let minors_count = knight_count + bishop_count;

        if minors_count < 2 {
            return true;
        }

        false
    }

    pub fn mobility_score(&self, square: Square, piece: Piece, color: Color) -> i32 {
        if piece == Piece::Pawn || piece == Piece::King {
            return 0;
        }

        let allied_pieces = self.board.occupancy(color);
        let combined = self.board.combined();

        let pawn_attacks = self.pawn_attacks(!color);

        let mobility = (match piece {
            Piece::Knight => get_knight_moves(square),
            Piece::Bishop => get_bishop_moves(square, combined),
            Piece::Rook => get_rook_moves(square, combined),
            Piece::Queen => get_bishop_moves(square, combined) | get_rook_moves(square, combined),
            _ => unreachable!(),
        } & !allied_pieces & !pawn_attacks).count_ones() as i32;

        match piece {
            Piece::Knight => s(mobility * 4, mobility * 4),
            Piece::Bishop => s(mobility * 5, mobility * 5),
            Piece::Rook => s(mobility * 2, mobility * 4),
            Piece::Queen => s(mobility, mobility * 2),
            _ => unreachable!(),
        }
    }

    pub fn calculate_game_phase(board: &Board) -> i32 {
        const TOTAL_PHASE: i32 = 24;

        let mut phase = 0;

        phase += board.pieces(Piece::Knight).count_ones() as i32;
        phase += board.pieces(Piece::Bishop).count_ones() as i32;
        phase += 2 * board.pieces(Piece::Rook).count_ones() as i32;
        phase += 4 * board.pieces(Piece::Queen).count_ones() as i32;

        let clamped = phase.min(TOTAL_PHASE);
        (256 * (TOTAL_PHASE - clamped) + 12) / TOTAL_PHASE
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
