use colored::Colorize;
use termcolor::{ColorChoice, StandardStream};

use crate::{check::Check, tile::Tile};

pub struct StdoutPrinter {
    allow_output: bool,
}

impl StdoutPrinter {
    pub fn new(allow_output: bool) -> Self {
        Self { allow_output }
    }

    pub fn transition(&self, tile: &Tile) {
        if self.allow_output {
            print_transition(tile)
        }
    }

    pub fn location_check(&self, check: &Check) {
        if self.allow_output {
            print_location_check(check)
        }
    }

    pub fn item_check(&self, check: &Check) {
        if self.allow_output {
            print_item_check(check)
        }
    }

    pub fn event(&self, event: &Check) {
        if self.allow_output {
            print_event(event)
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

pub fn print_transition(transition: &Tile) {
    println!(
        "Transition made!: time: {:?}, indoors: {:?}, to: {}",
        transition.timestamp,
        transition.indoors,
        format!("{}", transition.name).on_purple()
    );
}

pub fn print_location_check(check: &Check) {
    println!(
        "Check made! time: {:?}, location: {}",
        check.time_of_check,
        check.name.on_blue(),
    );
}

pub fn print_item_check(check: &Check) {
    if check.is_progressive {
        println!(
            "Item get! time: {:?}, item: {}",
            check.time_of_check,
            format!("{} - {}", check.name, check.progressive_level).on_green(),
        );
    } else {
        println!(
            "Item get! time: {:?}, item: {}",
            check.time_of_check,
            check.name.on_green(),
        );
    }
}

pub fn print_event(event: &Check) {
    if event.is_progressive {
        println!(
            "Event! time: {:?}, event: {}",
            event.time_of_check,
            format!("{} - {}", event.name, event.progressive_level).on_yellow(),
        );
    } else {
        println!(
            "Event! time: {:?}, item: {}",
            event.time_of_check,
            event.name.on_green(),
        );
    }
}
