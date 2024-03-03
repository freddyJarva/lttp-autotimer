use lttp_autotimer::write::CsvWriter;
use serde_json::to_string;
use tauri::Manager;

pub struct AppHandleWrapper {
    app_handle: tauri::AppHandle,
}

impl AppHandleWrapper {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl CsvWriter for AppHandleWrapper {
    fn write_event<S>(&mut self, record: S) -> anyhow::Result<()>
    where
        S: serde::Serialize {
            let serialized_data = to_string(&record)?;
            self.app_handle.emit_all("snes_event", serialized_data).expect("Should emit without fail");
            Ok(())
    }
}
