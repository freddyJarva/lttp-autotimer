// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod write;

use std::sync::Arc;

use app::{connect_to_sni, write::AppHandleWrapper, state::AppState};
use clap::{Arg, ArgMatches};
use lttp_autotimer::event::CommandState;
use tauri::Manager;
use tokio::sync::{mpsc, Mutex};

struct InputTx {
    inner: Mutex<mpsc::Sender<String>>,
}

fn get_args() -> ArgMatches {
    let app = clap::App::new("Rando Auto Timer")
        .arg(
            Arg::new("host")
                .long("host")
                .short('h')
                .help("url to server/localhost. When running locally the default value should be fine.")
                .takes_value(true)
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("update frequency")
                .long("freq")
                .short('f')
                .long_help("Interval in milliseconds the timer will check the snes memory for changes. Anything below ~15 will in practice be the same, as sni becomes the limiting rate factor.")
                .takes_value(true)
                .default_value("12")
        ).arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity for logging. can be set 0-2 times")
        ).arg(
            Arg::new("manual update")
                .long("manual")
                .short('m')
                .help("Only check for updates when user presses a key. Useful when debugging.")
        ).arg(
            Arg::new("Round times")
                .long("--round-times")
                .help("Show output on game events in app window. NOTE: This flag will have no effect when playing a race rom.")
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .help(
                    "port that sni server is listening on. It's most likely 8191",
                )
                .takes_value(true)
                .default_value("8191"),
        );
    app.get_matches()
}

async fn async_process_model(
    mut input_rx: mpsc::Receiver<String>,
    output_tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        while let Some(input) = input_rx.recv().await {
            let output = input;
            output_tx.send(output).await?;
        }
    }
}

#[tauri::command]
async fn timer_command(message: String, state: tauri::State<'_, InputTx>) -> Result<(), String>  {
    println!("Received frontend command: {}", message);
    let in_tx = state.inner.lock().await;
    in_tx.send(message).await.map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    let (in_tx, in_rx) = mpsc::channel::<String>(1);
    let (out_tx, mut out_rx) = mpsc::channel::<String>(1);

    tauri::Builder::default()
        .manage(InputTx {
            inner: Mutex::new(in_tx),
        })
        .invoke_handler(tauri::generate_handler![timer_command])
        .setup(|app| {
            let state = AppState {
                command: CommandState::None
            };
            let shared_state = Arc::new(Mutex::new(state));
            tauri::async_runtime::spawn(async move { async_process_model(in_rx, out_tx).await });

            let shared_state_frontend = Arc::clone(&shared_state);
            tauri::async_runtime::spawn(async move {
                loop {
                    if let Some(output) = out_rx.recv().await {
                        match output.as_str() {
                            "start_recording" => {
                                shared_state_frontend.lock().await.command = CommandState::StartRecording
                            },
                            "clear_event_log" => {
                                shared_state_frontend.lock().await.command = CommandState::ClearEventLog
                            }
                            _ => ()
                        }
                    }
                }
            });

            let snes_reader_handle = app.app_handle();
            let snes_reader_state = Arc::clone(&shared_state);
            tauri::async_runtime::spawn(async move {
                let args = get_args();

                let cfg = lttp_autotimer::CliConfig {
                    host: args.value_of("host").unwrap().to_string(),
                    port: args.value_of("port").unwrap().to_string(),
                    non_race_mode: true,
                    manual_update: args.is_present("manual update"),
                    update_frequency: args.value_of("update frequency").unwrap().parse().expect(
                        "specified update frequency (--freq/-f) needs to be a positive integer",
                    ),
                    _verbosity: args.occurrences_of("v"),
                    segment_run_mode: true,
                    round_times: args.is_present("Round times"),
                };
                let _ = connect_to_sni(cfg, snes_reader_handle, snes_reader_state).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
