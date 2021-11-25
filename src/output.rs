use colored::Colorize;
use termcolor::{ColorChoice, StandardStream};

/// Hack to make cmd.exe output colors instead of broken color escape codes
/// Not sure why it works since I use another crate for  coloring, but it does!
pub fn force_cmd_colored_output() {
    StandardStream::stdout(ColorChoice::Always);
}

/// Highlight delta changes between two array slices.
/// Will print changed values as (changed_idx, previous_value, new_value) sets.
/// This function assumes that left and right hand side arrays are the same size
pub fn print_verbose_diff<T: AsRef<[u8]>>(lhs: T, rhs: T) {
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
pub fn print_flags_toggled<T: AsRef<[u8]>>(lhs: T, rhs: T) {
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
