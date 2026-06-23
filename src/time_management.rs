use crate::search::Search;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Default)]
pub enum TimeManagement {
    #[default]
    None,
    MoveTime { time: usize },
    TimeLeft { time: usize },
}

impl TimeManagement {
    pub fn new(
        move_time: Option<usize>,
        time: Option<usize>,
        time_inc: Option<usize>,
    ) -> TimeManagement {
        if let Some(move_time) = move_time {
            TimeManagement::MoveTime {
                time: move_time.max(1),
            }
        } else if let Some(time) = time {
            TimeManagement::TimeLeft {
                time: (time / 20 + time_inc.unwrap_or(0) / 2).max(1),
            }
        } else {
            TimeManagement::None
        }
    }

    pub fn should_cancel_search(&self, search: &mut Search) -> bool {
        if search.think_timer.elapsed().as_millis() as usize >= self.time()
            && *self != TimeManagement::None
        {
            search.cancelled = true;
        }
        search.cancelled
    }

    pub fn time(&self) -> usize {
        match self {
            TimeManagement::MoveTime { time } | TimeManagement::TimeLeft { time } => *time,
            _ => 0,
        }
    }
}
