/*
*   Copyright (c) 2025 Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU Affero General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU Affero General Public License for more details.

*   You should have received a copy of the GNU Affero General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::thread::JoinHandle;

use actix_web::{dev::ServerHandle, web, Result};

use crate::services::{init_dweb_server, init_dweb_server_blocking};
use crate::StopHandle;
use dweb::client::DwebClientConfig;

#[derive(Debug)]
pub enum DwebServiceError {
    NOT_STARTED,
}
// TODO move to dweb-server::DwebService
pub struct DwebService {
    client_config: DwebClientConfig,
    is_started: bool,
    port: Option<u16>,
    stop: Option<web::Data<StopHandle>>,
    server: Option<JoinHandle<Result<(), std::io::Error>>>,
}

impl DwebService {
    pub fn new(client_config: DwebClientConfig) -> Self {
        DwebService {
            client_config,
            is_started: false,
            stop: None,
            server: None,
            port: None,
        }
    }

    /// Activate main dweb listener on the specified port
    pub fn start(&mut self, port: u16) {
        let dweb_client =
            match dweb::client::DwebClient::initialise_and_connect(&self.client_config).await {
                Ok(client) => client,
                Err(e) => {
                    println!("Failed to connect to Autonomi Network");
                    return;
                }
            };

        if let Some(active_port) = self.port {
            // If already listening at the correct port, we're good
            if active_port == port {
                return;
            }
        }

        self.port = Some(port);

        if port == 0 {
            println!("*** DwebService listener is disabled ***");
            self.is_started = false;
            return;
        }

        let stop_handle = self.stop.take();
        let server = self.server.take();

        if let Some(h) = stop_handle {
            h.stop(false);
        }

        if let Some(s) = server {
            if let Err(err) = s.join() {
                println!("DEBUG {err:?}");
            }
        }

        // Works with tauri Data BUT not actix_web::web::Data
        let stop_handle = web::Data::new(StopHandle::default());
        let cloned_stop_handle = stop_handle.clone();
        let mut client_config = self.client_config.clone();
        client_config.port = Some(port);
        self.server = Some(std::thread::spawn(async move || {
            crate::services::old_serve_with_ports(&DwebClientConfig::default(), None, false, false);
            Ok(())
        }));
        self.is_started = true;
        self.stop = Some(stop_handle);
    }

    pub async fn start_blocking(&mut self, port: u16) {
        if let Some(active_port) = self.port {
            // If already listening at the correct port, we're good
            if active_port == port {
                return;
            }
        }

        self.port = Some(port);

        if port == 0 {
            println!("*** DwebService listener is disabled ***");
            self.is_started = false;
            return;
        }

        let stop_handle = self.stop.take();
        let server = self.server.take();

        if let Some(h) = stop_handle {
            h.stop(false);
        }

        if let Some(s) = server {
            if let Err(err) = s.join() {
                println!("DEBUG {err:?}");
            }
        }

        // Works with tauri Data BUT not actix_web::web::Data
        let stop_handle = web::Data::new(StopHandle::default());
        let cloned_stop_handle = stop_handle.clone();
        let mut client_config = self.client_config.clone();
        client_config.port = Some(port);
        let _ = init_dweb_server_blocking(
            &client_config,
            None,
            Some(cloned_stop_handle),
            None,
            false,
            true,
        )
        .await;
        self.is_started = true;
        self.stop = Some(stop_handle);
    }
}

// #[derive(Default)]
// pub(crate) struct StopHandle {
//     inner: parking_lot::Mutex<Option<ServerHandle>>,
// }

// impl StopHandle {
//     /// Sets the server handle to stop.
//     pub(crate) fn register(&self, handle: ServerHandle) {
//         *self.inner.lock() = Some(handle);
//     }

//     /// Sends stop signal through contained server handle.
//     pub(crate) fn stop(&self, graceful: bool) {
//         if let Some(h) = self.inner.lock().as_ref() {
//             #[allow(clippy::let_underscore_future)]
//             let _ = h.stop(graceful);
//         }
//     }
// }
