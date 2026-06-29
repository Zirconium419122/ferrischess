use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Instant};

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

    pub fn should_cancel_search(&self, timer: Instant, cancelled: Arc<AtomicBool>) -> bool {
        if timer.elapsed().as_millis() as usize >= self.time()
            && *self != TimeManagement::None
        {
            cancelled.store(true, Ordering::Relaxed);
        }
        cancelled.load(Ordering::Relaxed)
    }

    pub fn time(&self) -> usize {
        match self {
            TimeManagement::MoveTime { time } | TimeManagement::TimeLeft { time } => *time,
            _ => 0,
        }
    }
}
