/*
 Copyright (c) 2025 Mark Hughes

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU Affero General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU Affero General Public License for more details.

 You should have received a copy of the GNU Affero General Public License
 along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use std::sync::Mutex;

use dweb::client::DwebClientConfig;
use dweb_server::DwebService;
use tauri::{Manager, State};

#[tauri::command]
fn start_server(state: State<'_, ServerState>, port: u16) {
    let mut dweb_service = state.dweb_service.lock().unwrap();
    dweb_service.start(port, None);
}

#[tauri::command]
fn dweb_open(_state: State<'_, ServerState>, address_name_or_link: String) {
    let main_server = "http://127.0.0.1:5537";
    let url = format!("{main_server}/dweb-open/{address_name_or_link}");
    println!("dweb_open() opening {url}");
    let _ = open::that(url);
}

struct ServerState {
    dweb_service: Mutex<DwebService>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            // Make builtin names such as 'awesome' available (in addition to opening xor addresses)
            dweb::web::name::register_builtin_names(false);
            // Set up dweb service
            app.manage(ServerState {
                dweb_service: Mutex::new(DwebService::new(DwebClientConfig::default())),
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_server, dweb_open])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
