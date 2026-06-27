use chessframe::chess_move::ChessMove;

use crate::search::Bound;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Entry {
    pub zobrist: u64,
    pub depth: u8,
    pub score: i32,
    pub mv: ChessMove,
    pub bound: Bound,
}

impl Entry {
    pub fn empty() -> Entry {
        Entry {
            zobrist: 0,
            depth: 0,
            score: 0,
            bound: Bound::None,
            mv: ChessMove::NULL_MOVE,
        }
    }
}

pub struct TranspositionTable {
    entries: Vec<Entry>,
    max_entries: usize,
}

impl TranspositionTable {
    pub fn with_capacity(num_entries: usize) -> TranspositionTable {
        let size = num_entries.next_power_of_two();

        TranspositionTable {
            entries: vec![Entry::empty(); size],
            max_entries: size,
        }
    }

    pub fn with_size_mb(size_mb: usize) -> TranspositionTable {
        let entry_size = std::mem::size_of::<Entry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;

        Self::with_capacity(num_entries)
    }

    fn index(&self, zobrist: u64) -> usize {
        (zobrist as usize) & (self.max_entries - 1)
    }

    pub fn store(&self, zobrist: u64, depth: u8, score: i32, mv: ChessMove, bound: Bound) {
        let index = self.index(zobrist);

        let entry = unsafe { self.entries.as_ptr().add(index) as *mut Entry };

        unsafe {
            if (*entry).zobrist == zobrist {
                if (*entry).depth <= depth {
                    *entry = Entry {
                        zobrist,
                        depth,
                        score,
                        mv,
                        bound,
                    };
                }
            } else {
                *entry = Entry {
                    zobrist,
                    depth,
                    score,
                    mv,
                    bound,
                };
            }
        }
    }

    pub fn probe(&self, zobrist: u64) -> Option<Entry> {
        let index = self.index(zobrist);

        let entry = self.entries[index];
        if entry != Entry::empty() && entry.zobrist == zobrist {
            Some(entry)
        } else {
            None
        }
    }
}
