use chrono::{DateTime, Duration, Timelike, Utc};
use colored::{ColoredString, Colorize};
use termcolor::{ColorChoice, StandardStream};

use crate::{check::Check, tile::Tile, event::EventEnum, time::{TimeVerdict, RunStatistics}};

pub trait TimeFormat {
    fn fmt_avg(&self) -> String;
    fn fmt_rolling_avg(&self, n: usize) -> String;
    fn fmt_new_time(&self, new_time: &Duration) -> String;
}

impl <T>TimeFormat for T
where 
    T: RunStatistics 
{
    fn fmt_avg(&self) -> String {
        format!("avg: {}", format_duration(self.avg()))
    }

    fn fmt_rolling_avg(&self, n: usize) -> String {
        format!("rolling_avg ({}): {}", n, format_duration(self.rolling_avg(n)))
    }

    fn fmt_new_time(&self, new_time: &Duration) -> String {
        match self.run_time_verdict(new_time) {
            TimeVerdict::Bad(diff) => format!("Finished in {} (+ {})", format_red_duration(*new_time), format_red_duration(diff)),
            TimeVerdict::Ok(skew) => format!("Finished in {} (± {})", format_duration(*new_time), format_duration(skew)),
            TimeVerdict::Best(diff) => format!("Finished in {} (- {})", format_gold_duration(*new_time), format_gold_duration(diff)),
        }
    }
}

pub struct StdoutPrinter {
    allow_output: bool,
    previous_time: DateTime<Utc>,
}

impl StdoutPrinter {
    pub fn new(allow_output: bool) -> Self {
        Self {
            allow_output,
            previous_time: Utc::now(),
        }
    }

    pub fn debug<S: AsRef<str>>(&self, s: S) {
        if self.allow_output {
            println!("{}", s.as_ref());
        }
    }

    pub fn transition(&mut self, tile: &Tile) {
        if self.allow_output {
            print_transition(tile, &self.previous_time)
        }
        self.previous_time = tile.timestamp.unwrap()
    }

    pub fn location_check(&mut self, check: &Check) {
        if self.allow_output {
            print_location_check(check, &self.previous_time)
        }
        self.previous_time = check.time_of_check.unwrap()
    }

    pub fn item_check(&mut self, check: &Check) {
        if self.allow_output {
            print_item_check(check, &self.previous_time)
        }
        self.previous_time = check.time_of_check.unwrap()
    }

    pub fn event(&mut self, event: &Check) {
        if self.allow_output {
            print_event(event, &self.previous_time)
        }
        self.previous_time = event.time_of_check.unwrap()
    }

    pub fn action(&mut self, event: &Check) {
        if self.allow_output {
            print_action(event)
        }
    }

    pub fn command(&mut self, event: &Check) {
        if self.allow_output {
            print_command(event)
        }
    }

    pub fn segment_finish(&mut self, objectives: &[(EventEnum, DateTime<Utc>)]) {
        if self.allow_output {
            print_segment_finish(objectives)
        }
    }
}

/// Hack to make cmd.exe output colors instead of broken color escape codes
/// Not sure why it works since I use another crate for  coloring, but it does!
pub fn force_cmd_colored_output() {
    StandardStream::stdout(ColorChoice::Always);
}

/// Highlight delta changes between two array slices.
/// Will print changed values as (changed_idx, previous_value, new_value) sets.
/// This function assumes that left and right hand side arrays are the same size
pub fn print_verbose_diff<T: AsRef<[u8]>, U: AsRef<[u8]>>(lhs: T, rhs: U) {
    let lhs = lhs.as_ref();
    let rhs = rhs.as_ref();
    print!("delta changes (changed_idx, previous_value, new_value): ");
    for i in 0..lhs.len() {
        if lhs[i] != rhs[i] {
            print!(
                "({}, {}, {}) ",
                i,
                lhs[i].to_string().red(),
                rhs[i].to_string().green()
            )
        }
    }
    print!("\n");
}

/// What's considered flags - according to this function - are boolean values, i.e. values that are either 0 or 1
pub fn print_flags_toggled<T: AsRef<[u8]>, U: AsRef<[u8]>>(lhs: T, rhs: U) {
    let lhs = lhs.as_ref();
    let rhs = rhs.as_ref();
    print!("flags toggled (changed_idx, previous_value, new_value): ");
    for i in 0..lhs.len() {
        if lhs[i] as u32 + rhs[i] as u32 == 1 {
            print!(
                "({}, {}, {}) ",
                i,
                lhs[i].to_string().red(),
                rhs[i].to_string().green()
            )
        }
    }
    print!("\n");
}

pub fn print_transition(transition: &Tile, previous_time: &DateTime<Utc>) {
    print_trigger(
        format!("{}", transition.name).on_purple(),
        &transition.timestamp.unwrap(),
        previous_time,
    );
}

pub fn print_location_check(check: &Check, previous_time: &DateTime<Utc>) {
    print_trigger(
        check.name.on_blue(),
        &check.time_of_check.unwrap(),
        previous_time,
    );
}

pub fn print_item_check(check: &Check, previous_time: &DateTime<Utc>) {
    if check.is_progressive {
        print_trigger(
            format!("{} - {}", check.name, check.progressive_level).on_green(),
            &check.time_of_check.unwrap(),
            previous_time,
        );
    } else {
        print_trigger(
            check.name.on_green(),
            &check.time_of_check.unwrap(),
            previous_time,
        );
    }
}

pub fn print_event(check: &Check, previous_time: &DateTime<Utc>) {
    if check.is_progressive {
        print_trigger(
            format!("{} - {}", check.name, check.progressive_level).on_yellow(),
            &check.time_of_check.unwrap(),
            previous_time,
        );
    } else {
        print_trigger(
            check.name.on_yellow(),
            &check.time_of_check.unwrap(),
            previous_time,
        );
    }
}

pub fn print_action(check: &Check) {
    println!(
        "{}",
        format!("{} - {}", check.name, check.progressive_level).dimmed()
    );
}

pub fn print_command(check: &Check) {
    println!(
        "{}",
        format!("{} - {}", check.name, check.progressive_level).yellow()
    );
}

pub fn print_segment_finish(objectives: &[(EventEnum, DateTime<Utc>)]) {
    let start = &objectives[0];
    let end = &objectives[objectives.len() - 1];
    println!(
        "{}",
        format!("Segment Time - {}", format_duration(end.1 - start.1)).green()
    );
}

fn print_trigger(
    trigger_text: ColoredString,
    current_time: &DateTime<Utc>,
    previous_time: &DateTime<Utc>,
) {
    println!(
        "{}, delta: {}, time: {:02}:{:02}:{:02}",
        trigger_text,
        format_duration(current_time.time() - previous_time.time()),
        current_time.hour(),
        current_time.minute(),
        current_time.second()
    )
}


pub fn format_duration(time: Duration) -> ColoredString {
    format!("{:.3}", duration_to_float(time)).cyan()
}

pub fn format_gold_duration(time: Duration) -> ColoredString {
    format!("{:.3}", duration_to_float(time)).bright_yellow()
}

pub fn format_red_duration(time: Duration) -> ColoredString {
    format!("{:.3}", duration_to_float(time)).red()
}

fn duration_to_float(time: Duration) -> f64 {
    time.to_string()
        .strip_prefix("PT")
        .unwrap_or_default()
        .strip_suffix("S")
        .unwrap_or_default()
        .parse()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;
    #[test]
    fn test_format_duration() {
        // Given
        let past = NaiveTime::from_hms_nano(0, 0, 0, 0);
        let present = NaiveTime::from_hms_nano(0, 0, 20, 133700000);
        let actual = format_duration(present - past);
        assert_eq!(actual, "20.134".cyan())
    }

    #[test]
    fn test_duration_to_float() {
        // Given
        let past = NaiveTime::from_hms_nano(0, 0, 0, 0);
        let present = NaiveTime::from_hms_nano(0, 0, 20, 133700000);
        let actual = duration_to_float(present - past);
        assert_eq!(actual, 20.1337)
    }
}
