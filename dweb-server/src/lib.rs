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
mod services;
mod web_extras;

use std::thread::JoinHandle;

use actix_web::{
    dev::ServerHandle,
    get,
    http::StatusCode,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Result,
};

use crate::services::init_dweb_server;
use dweb::client::{DwebClient, DwebClientConfig};

#[derive(Debug)]
pub enum DwebServiceError {
    NOT_STARTED,
}
// TODO move to dweb-server::DwebService
pub struct DwebService {
    client_config: DwebClientConfig,
    dweb_client: Option<DwebClient>,
    is_started: bool,
    port: Option<u16>,
    stop: Option<web::Data<StopHandle>>,
    server: Option<JoinHandle<Result<(), std::io::Error>>>,
}

impl DwebService {
    pub fn new(client_config: DwebClientConfig) -> Self {
        DwebService {
            client_config,
            dweb_client: None,
            is_started: false,
            stop: None,
            server: None,
            port: None,
        }
    }

    /// Activate main dweb listener on the specified port
    pub fn start(&mut self, port: u16) {
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
        let client_config = self.client_config.clone();
        self.server = Some(std::thread::spawn(move || {
            init_dweb_server(port, &client_config, Some(cloned_stop_handle))
        }));
        self.is_started = true;
        self.stop = Some(stop_handle);
    }
}

#[actix_web::main]
async fn init_demo_server(
    port: u16,
    stop_handle: Option<actix_web::web::Data<StopHandle>>,
) -> Result<(), std::io::Error> {
    let http_server = HttpServer::new(move || App::new().service(handle_get).service(handle_spawn))
        .bind(("127.0.0.1", port));

    match http_server {
        Ok(server) => {
            println!("*** Started DwebService listener at 127.0.0.1:{port} ***");
            let running_server = server.run();
            if let Some(stop_handle) = stop_handle {
                stop_handle.register(running_server.handle());
            }
            running_server.await
        }
        Err(err) => {
            eprintln!("Unable to start server at http://127.0.0.1:{port}, {err}");
            Err(err)
        }
    }
}

#[get("/")]
async fn handle_get(_req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponseBuilder::new(StatusCode::OK).body("Bingo!"))
}

#[get("/spawn/{port}")]
async fn handle_spawn(
    _req: HttpRequest,
    port: actix_web::web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let port = port.into_inner().parse().unwrap_or(9999);
    // std::thread::spawn(move || init_demo_server(port, None));

    // TODO inherit client config from main server using Actix data
    std::thread::spawn(move || init_dweb_server(port, &DwebClientConfig::default(), None));

    let message = format!("Spawed server on port: {port}");
    Ok(HttpResponseBuilder::new(StatusCode::OK).body(message))
}

#[derive(Default)]
pub(crate) struct StopHandle {
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
