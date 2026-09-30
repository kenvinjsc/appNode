// AIC CAD desktop shell. The same `Engine::dispatch_json` used by the dev
// server is exposed as a single Tauri command; the core runs in-process.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aic_api::Engine;
use std::sync::Mutex;

struct Core(Mutex<Engine>);

/// UI → core. Heavy requests (nesting, toolpaths) run on Tauri's async pool
/// so the webview never blocks.
#[tauri::command]
async fn dispatch(state: tauri::State<'_, Core>, request: String) -> Result<String, String> {
    let mut engine = state.0.lock().map_err(|_| "core poisoned".to_string())?;
    Ok(engine.dispatch_json(&request))
}

fn main() {
    tauri::Builder::default()
        .manage(Core(Mutex::new({
            let mut e = Engine::new();
            e.set_library_path(aic_api::library::default_library_path());
            e
        })))
        .invoke_handler(tauri::generate_handler![dispatch])
        .run(tauri::generate_context!())
        .expect("error while running AIC CAD");
}
