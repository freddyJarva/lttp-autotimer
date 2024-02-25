use clap::Arg;

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
                .about("Interval in milliseconds the timer will check the snes memory for changes. Anything below ~15 will in practice be the same, as sni becomes the limiting rate factor.")
                .takes_value(true)
                .default_value("12")
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
        ).arg(
            Arg::new("Round times")
                .long("--round-times")
                .about("Show output on game events in app window. NOTE: This flag will have no effect when playing a race rom.")
        ).arg(
            Arg::new("Segment run mode")
                .long("--segment-mode")
                .short('s')
                .about("Where to end the timer for segments")
        );

    let matches;

    force_cmd_colored_output();

    // Hacky way to ensure correct default port depending on which feature flag is set
    #[cfg(feature = "sni")]
    {
        matches = app
            .arg(
                Arg::new("port")
                    .long("port")
                    .short('p')
                    .about(
                        "port that websocket server is listening on. For sni it's most likely 8191",
                    )
                    .takes_value(true)
                    .default_value("8191"),
            )
            .get_matches();
        println!("Running in SNI performance mode");
        connect_to_sni(&matches)?;
    }
    Ok(())
}
