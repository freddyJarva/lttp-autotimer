use chrono::Duration;

pub trait SequenceStatistics {
    fn avg(&self) -> chrono::Duration;
    fn rolling_avg(&self, n: usize) -> chrono::Duration;
}

impl SequenceStatistics for Vec<Duration> {
    fn avg(&self) -> chrono::Duration {
        let sum: i64 = self.iter().map(|&d| d.num_milliseconds()).sum();
        let avg = sum / (self.len() as i64);
        Duration::milliseconds(avg)
    }

    fn rolling_avg(&self, n: usize) -> Duration {
        let sum: i64 = self
            .iter()
            .rev()
            .take(n)
            .map(|&d| d.num_milliseconds())
            .sum();
        let avg = sum / (usize::min(n, self.len()) as i64);
        Duration::milliseconds(avg)
    }
}
