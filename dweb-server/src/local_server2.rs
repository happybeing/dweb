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

use std::thread::JoinHandle;

use actix_web::{
    dev::ServerHandle,
    get,
    http::StatusCode,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Result,
};

// TODO move to dweb-server::DwebService
pub struct DemoService {
    port: Option<u16>,
    stop: Option<web::Data<StopHandle>>,
    server: Option<JoinHandle<Result<(), std::io::Error>>>,
}

impl DemoService {
    pub fn new() -> Self {
        DemoService {
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
            println!("*** DemoService listener is disabled ***");
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

        let stop_handle = web::Data::new(StopHandle::default());
        let cloned_stop_handle = stop_handle.clone();

        self.server = Some(std::thread::spawn(move || {
            init_dweb_server(port, Some(cloned_stop_handle))
        }));
        self.stop = Some(stop_handle);
    }
}

#[actix_web::main]
async fn init_dweb_server(
    port: u16,
    stop_handle: Option<Data<StopHandle>>,
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
    port: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let port = port.into_inner().parse().unwrap_or(9999);
    std::thread::spawn(move || init_dweb_server(port, None));

    let message = format!("Spawed server on port: {port}");
    Ok(HttpResponseBuilder::new(StatusCode::OK).body(message))
}

#[derive(Default)]
struct StopHandle {
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
