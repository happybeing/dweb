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

mod helpers;
pub mod services;
mod web_extras; // TODO remove

use std::thread::JoinHandle;

use actix_web::{dev::ServerHandle, web, Result};

use crate::services::{init_dweb_server_blocking, init_dweb_server_non_blocking};
use dweb::client::{DwebClient, DwebClientConfig};

// Note: some of this code was modelled on the OAuth2PkceService implementation
// in https://github.com/apicize/app/blob/64ca56852aea48032f8125a674f6af47eb56f9a4/app/src-tauri/src/pkce.rs#L240
// The StopHandle feature been left in place in case they are wanted later, but is not yet used by dweb.
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

    /// Start the main dweb server on the specified port (non-blocking)
    ///
    /// This spawns a thread for the server and returns immediately.
    /// Use this from apps where you need a non-blocking server.
    ///
    /// For an example Tauri app using this, see https://codeberg.org:happybeing/dweb-server-tauri-app
    pub fn start(&mut self, port: u16, dweb_client: Option<DwebClient>) {
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
        self.server = Some(std::thread::spawn(move || {
            init_dweb_server_non_blocking(
                &client_config,
                dweb_client,
                Some(cloned_stop_handle),
                None,
            )
        }));
        self.is_started = true;
        self.stop = Some(stop_handle);
    }

    /// Start the main dweb server on the specified port (blocking)
    ///
    /// This starts the server and waits for it to exit.
    /// Use this to run a server where you don't have other threads, such as a CLI
    ///
    /// For an example CLI app using this, see https://codeberg.org:happybeing/dweb/dweb-cli
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
        self.is_started = true;
        self.stop = Some(stop_handle);
        let _ =
            init_dweb_server_blocking(&client_config, None, Some(cloned_stop_handle), None).await;
    }
}

#[derive(Default)]
pub struct StopHandle {
    inner: parking_lot::Mutex<Option<ServerHandle>>,
}

impl StopHandle {
    /// Sets the server handle to stop.
    pub(crate) fn register(&self, handle: ServerHandle) {
        *self.inner.lock() = Some(handle);
    }

    /// Sends stop signal through contained server handle.
    pub(crate) fn stop(&self, graceful: bool) {
        if let Some(h) = self.inner.lock().as_ref() {
            #[allow(clippy::let_underscore_future)]
            let _ = h.stop(graceful);
        }
    }
}
