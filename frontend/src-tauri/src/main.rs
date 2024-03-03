// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use tauri::Manager;
use tokio::sync::{mpsc, Mutex};

struct InputTx {
    inner: Mutex<mpsc::Sender<String>>
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
fn rs2js<R: tauri::Runtime>(message: String, manager: &impl Manager<R>) {
    println!("THIS IS RUST rs2js {}", message);
    manager
        .emit_all("rs2js", format!("rs: {}", message))
        .unwrap();
}


#[tauri::command]
async fn js2rs(
    message: String,
    state: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    println!("THIS IS RUST js2rs {:?}", message);
    let in_tx = state.inner.lock().await;
    in_tx.send(message).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
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
    .invoke_handler(tauri::generate_handler![greet])
    .invoke_handler(tauri::generate_handler![js2rs])
    .setup(|app| {
        tauri::async_runtime::spawn(async move {
            async_process_model(in_rx, out_tx).await
        });

        let app_handle = app.handle();
        tauri::async_runtime::spawn(async move {
            loop {
                if let Some(output) = out_rx.recv().await {
                    rs2js(output, &app_handle);
                }
            }
        });

        let snes_reader_handle = app.app_handle();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                snes_reader_handle.emit_all("snes_event", "hello world!")
                    .expect("Should never fail to send message");
            }
        });

        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
