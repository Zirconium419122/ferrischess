use chessframe::{board::Board, chess_move::ChessMove, piece::Piece, square::Square};

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

    pub fn update_history(&mut self, to: Square, piece: Piece, value: i16) {
        let entry = &mut self.history[piece.to_index()][to.to_index()];
        *entry = (*entry + value).clamp(-10_000, 10_000)
    }

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
            return 200_000;
        }

        if Some(mv) == tt_move {
            return 100_000;
        }

        if let Some(captured) = board.get_piece(mv.to) {
            let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };

            return 50_000 + Self::get_mvv_lva(captured, moved) as i32;
        }

        if ply < KILLER_MOVE_COUNT as u8 && self.killer_moves[ply as usize] == mv {
            return 20_000;
        }

        let moved = unsafe { board.get_piece(mv.from).unwrap_unchecked() };
        self.history[moved.to_index()][mv.to.to_index()] as i32 / 2
    }

    #[inline]
    fn get_mvv_lva(victim: Piece, attacker: Piece) -> i8 {
        unsafe { *MVV_LVA.get_unchecked(victim.to_index() * 6 + attacker.to_index()) }
    }
}
