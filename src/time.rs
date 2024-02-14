use chrono::{Duration, DateTime, Utc};

use crate::event::EventEnum;

pub trait RunStatistics {
    fn objective_time_min(&self, objective_idx: usize) -> Option<Duration>;
    fn run_time_min(&self) -> Option<Duration>;
    fn new_best_objective_time(&self, objective_idx: usize, new_time: &Duration) -> bool;
    fn avg(&self) -> Duration;
    fn rolling_avg(&self, n: usize) -> Duration;
    fn objective_time_verdict(&self, objective_idx: usize, new_time: &Duration) -> TimeVerdict;
    fn run_time_verdict(&self, new_time: &Duration) -> TimeVerdict;
}

impl RunStatistics for Vec<Vec<(EventEnum, DateTime<Utc>)>> {
    fn avg(&self) -> Duration {
        let sum: i64 = self.iter().map(|run| {
            run.to_duration().num_milliseconds()
        }).sum();
        let avg = sum / (self.len() as i64);
        Duration::milliseconds(avg)
    }

    fn rolling_avg(&self, n: usize) -> Duration {
        let sum: i64 = self.iter()
            .map(|run| {
                run.to_duration().num_milliseconds()
            })
            .take(n)
            .sum();
        let avg = sum / (usize::min(n, self.len()) as i64);
        Duration::milliseconds(avg)
    }

    fn objective_time_min(&self, objective_idx: usize) -> Option<Duration> {
        self.iter()
            .filter_map(|run| {
                run.objective_duration(objective_idx)
            })
            .min()
    }

    fn run_time_min(&self) -> Option<Duration> {
        self.iter()
            .map(|run| run.to_duration())
            .min()
    }

    fn new_best_objective_time(&self, objective_idx: usize, new_time: &Duration) -> bool {
        match self.objective_time_min(objective_idx) {
            Some(ref previous_best) => new_time < previous_best,
            None => true,
        }
    }

    fn objective_time_verdict(&self, objective_idx: usize, new_time: &Duration) -> TimeVerdict {
        match self.objective_time_min(objective_idx) {
            Some(ref previous_best) => time_verdict(previous_best, new_time),
            None => TimeVerdict::Best(Duration::milliseconds(0)),
        }
    }

    fn run_time_verdict(&self, new_time: &Duration) -> TimeVerdict {
        match self.run_time_min() {
            Some(ref previous_best) => time_verdict(previous_best, new_time),
            None => TimeVerdict::Best(Duration::milliseconds(0)),
        }
    }
}

const TIME_SKEW_MILLI: i64 = 38;
fn time_verdict(previous_best: &Duration, new_time: &Duration) -> TimeVerdict {
    let time_skew = Duration::milliseconds(TIME_SKEW_MILLI);
    let previous_lo_skew = *previous_best - time_skew;
    let previous_hi_skew = *previous_best + time_skew;
    if *new_time < previous_lo_skew {
        return TimeVerdict::Best(*previous_best - *new_time)
    } else if *new_time < previous_hi_skew {
        return TimeVerdict::Ok(time_skew) 
    } else {
        // flipped because we want absolute values
        return TimeVerdict::Bad(*new_time - *previous_best)
    }
}

pub trait SingleRunStats {
    fn to_duration(&self) -> Duration;
    fn objective_duration(&self, objective_idx: usize) -> Option<Duration>;
}

impl SingleRunStats for Vec<(EventEnum, DateTime<Utc>)> {
    fn to_duration(&self) -> Duration {
        self[self.len() - 1].1 - self[0].1
    }

    fn objective_duration(&self, objective_idx: usize) -> Option<Duration> {
        if objective_idx == 0 || objective_idx >= self.len() {
            println!("objective_idx must be > 0 and < Vec length");
            return None
        }
        Some(self[objective_idx].1 - self[objective_idx - 1].1)
    }
}

pub enum TimeVerdict {
    Bad(Duration),
    Ok(Duration),
    Best(Duration)
}
