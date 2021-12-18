use clap::Arg;
#[cfg(feature = "qusb")]
use lttp_autotimer::connect_to_qusb;

use lttp_autotimer::output::force_cmd_colored_output;

#[cfg(feature = "sni")]
use lttp_autotimer::connect_to_sni;

fn main() -> anyhow::Result<()> {
    let app = clap::App::new("Rando Auto Timer")
        .arg(
            Arg::new("host")
                .long("host")
                .short('h')
                .about("url to server/localhost. When running locally the default value should be fine.")
                .takes_value(true)
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("update frequency")
                .long("freq")
                .short('f')
                .about("Interval in milliseconds the timer will check the snes memory for changes. Default is about 60 times per second")
                .takes_value(true)
                .default_value("16")
        ).arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .about("Sets the level of verbosity for logging. can be set 0-2 times")
        ).arg(
            Arg::new("manual update")
                .long("manual")
                .short('m')
                .about("Only check for updates when user presses a key. Useful when debugging.")
        ).arg(
            Arg::new("Non race mode")
                .long("--non-race")
                .about("Show output on game events in app window. NOTE: This flag will have no effect when playing a race rom.")
        );

    let matches;

    force_cmd_colored_output();

    // Hacky way to ensure correct default port depending on which feature flag is set
    #[cfg(feature = "qusb")]
    {
        matches = app.arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .about("port that websocket server is listening on. For qusb it's most likely 8080")
                .takes_value(true)
                .default_value("8080"),
        ).get_matches();
        connect_to_qusb(&matches)?;
    }
    #[cfg(feature = "sni")]
    {
        matches = app.arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .about("port that websocket server is listening on. For qusb it's most likely 8080")
                .takes_value(true)
                .default_value("8191"),
        ).get_matches();
        connect_to_sni(&matches)?;
    }
    Ok(())
}
