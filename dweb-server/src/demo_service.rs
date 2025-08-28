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

use std::io;
use std::thread::JoinHandle;
use std::time::Duration;

use actix_web::{
    dev::{ServerHandle, Service},
    get,
    http::StatusCode,
    middleware::Logger,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer,
};
use color_eyre::eyre::Result;
use utoipa::OpenApi;
use utoipa_actix_web::scope::scope;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

use dweb::cache::directory_with_port::DirectoryVersionWithPort;
use dweb::{
    client::{DwebClient, DwebClientConfig},
    web::SERVER_PORTS_MAIN_PORT,
};

// TODO move to dweb-server::DwebService
pub struct DemoService {
    client_config: DwebClientConfig,
    port: Option<u16>,
    stop: Option<web::Data<StopHandle>>,
    server: Option<JoinHandle<Result<(), std::io::Error>>>,
}

// Original
// impl DemoService {
//     // pub fn new() -> Self {
//     //     DemoService {
//     //         stop: None,
//     //         server: None,
//     //         port: None,
//     //     }
//     // }

//     pub fn new(client_config: DwebClientConfig) -> Self {
//         DemoService {
//             client_config,
//             dweb_client: None,
//             stop: None,
//             server: None,
//             port: None,
//         }
//     }

//     /// Activate main dweb listener on the specified port
//     pub fn start(&mut self, port: u16) {
//         if let Some(active_port) = self.port {
//             // If already listening at the correct port, we're good
//             if active_port == port {
//                 return;
//             }
//         }

//         self.port = Some(port);

//         if port == 0 {
//             println!("*** DemoService listener is disabled ***");
//             return;
//         }

//         let stop_handle = self.stop.take();
//         let server = self.server.take();

//         if let Some(h) = stop_handle {
//             h.stop(false);
//         }

//         if let Some(s) = server {
//             if let Err(err) = s.join() {
//                 println!("DEBUG {err:?}");
//             }
//         }

//         let stop_handle = web::Data::new(StopHandle::default());
//         let cloned_stop_handle = stop_handle.clone();

//         self.server = Some(std::thread::spawn(move || {
//             init_dweb_server(port, Some(cloned_stop_handle))
//         }));
//         self.stop = Some(stop_handle);
//     }
// }

impl DemoService {
    pub fn new(client_config: DwebClientConfig) -> Self {
        DemoService {
            client_config,
            stop: None,
            server: None,
            port: None,
        }
    }

    /// Must be used to create the DwebClient prior to calling start()
    // pub async fn ensure_connection(&mut self) -> Result<DwebClient> {
    //     match self.dweb_client.clone() {
    //         Some(dweb_client) => Ok(dweb_client.clone()),
    //         None => {
    //             let dweb_client =
    //                 match dweb::client::DwebClient::initialise_and_connect(&self.client_config)
    //                     .await
    //                 {
    //                     Ok(dweb_client) => {
    //                         self.dweb_client = Some(dweb_client.clone());
    //                         dweb_client
    //                     }
    //                     Err(e) => {
    //                         let message = format!("Failed to connect to Autonomi Network: {e}");
    //                         println!("DEBUG: {message}");
    //                         return Err(eyre!("{message}"));
    //                     }
    //                 };
    //             self.dweb_client = Some(dweb_client.clone());
    //             Ok(dweb_client)
    //         }
    //     }
    // }

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
        let mut client_config = self.client_config.clone();
        client_config.port = Some(port);
        self.server = Some(std::thread::spawn(move || {
            tauri_init_dweb_server(
                &client_config,
                None,
                Some(cloned_stop_handle),
                None,
                false,
                false,
            )
        }));
        self.stop = Some(stop_handle);
    }
}

use std::io::Error;
use std::io::ErrorKind::NotConnected;

use crate::services::{api_dweb, api_dweb_ant, openapi, www};

// This works with a Tauri start server, and when called from a tweaked dweb_open
// Now making it suitable for CLI (in services::DwebService::init_dweb_service())
#[actix_web::main]
pub async fn tauri_init_dweb_server(
    client_config: &DwebClientConfig,
    dweb_client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    spawn_server: bool,
    start_blocking: bool,
) -> Result<(), std::io::Error> {
    println!("DDDDDD tauri_init_dweb_server()...");
    let client = if let Some(dweb_client) = dweb_client {
        dweb_client
    } else {
        match dweb::client::DwebClient::initialise_and_connect(&client_config).await {
            Ok(dweb_client) => dweb_client,
            Err(e) => {
                let message = format!("Failed to connect to Autonomi Network: {e}");
                println!("DEBUG: {message}");
                return Err(Error::new(NotConnected, e));
            }
        }
    };

    let directory_version_with_port_copy1 = directory_version_with_port.clone();
    let directory_version_with_port_copy2 = directory_version_with_port.clone();
    let directory_version_with_port = directory_version_with_port;

    let (history_address, archive_address) =
        if let Some(directory_version_with_port) = directory_version_with_port_copy1 {
            (
                directory_version_with_port.history_address,
                Some(directory_version_with_port.archive_address),
            )
        } else {
            (None, None)
        };

    // TODO control logger using CLI? (this enables Autonomi and HttpRequest logging to terminal)
    // env_logger::init_from_env(Env::default().default_filter_or("info"));

    let is_main_server = !spawn_server;
    let host = client.host.clone();
    let port = client_config.port.unwrap_or(SERVER_PORTS_MAIN_PORT);
    let client = client.clone();

    // let http_server = HttpServer::new(move || App::new().service(handle_get).service(handle_spawn))
    //     .bind(("127.0.0.1", port));

    // match http_server {
    //     Ok(server) => {
    //         println!("*** Started DwebService listener at 127.0.0.1:{port} ***");
    //         let running_server = server.run();
    //         if let Some(stop_handle) = stop_handle {
    //             stop_handle.register(running_server.handle());
    //         }
    //         running_server.await
    //     }
    //     Err(err) => {
    //         eprintln!("Unable to start server at http://127.0.0.1:{port}, {err}");
    //         Err(err)
    //     }
    // }

    println!("DDDDDD HttpServer::new()...");
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                actix_cors::Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method()
                    .expose_any_header()
                    .send_wildcard(),
            )
            // Macro logging using env_logger for both actix and libs such as Autonomi
            .wrap(Logger::default())
            // Log Requests and Responses to terminal
            .wrap_fn(|req, srv| {
                println!(
                    "DEBUG serve with ports HttpRequest : {} {}",
                    req.head().method,
                    req.path()
                );
                let fut = srv.call(req);
                async {
                    let res = fut.await?;

                    let reason = res.response().head().reason();
                    let reason = if !reason.is_empty() {
                        if res.response().head().reason() != "OK" {
                            &format!(" ({})", res.response().head().reason())
                        } else {
                            ""
                        }
                    } else {
                        ""
                    };
                    println!(
                        "DEBUG serve with ports HttpResponse: {} {}",
                        res.status(),
                        reason
                    );

                    Ok(res)
                }
            })
            .into_utoipa_app()
            .openapi(openapi::DwebApiDoc::openapi())
            // Testing to see what logging this provides
            .map(|app| app.wrap(actix_web::middleware::Logger::default()))
            // TODO consider using utoipa_actix_web::configure() to separate service configuration into their respective modules
            .service(handle_spawn) // Testing
            .service(crate::services::www::dweb_open::dweb_open)
            .service(www::dweb_open::dweb_open_as)
            .service(www::dweb_info::dweb_info)
            .service(www::dweb_version::dweb_version)
            .service(api_dweb::v0::ant_proxy_id)
            // Autonomi APIs
            // .service(
            //     scope(dweb::api::ANT_API_ROUTE)
            //     // TODO - replicate /ant-0 support using a library (to be provided by @Traktion from AntTP)
            // )
            .service(
                scope(dweb::api::DWEB_API_ROUTE)
                    // dweb Enhanced Automonomi APIs
                    .service(api_dweb_ant::v0::archive::archive_post_public)
                    .service(api_dweb_ant::v0::archive::archive_post_private)
                    .service(api_dweb_ant::v0::archive::archive_get)
                    .service(api_dweb_ant::v0::archive::archive_get_version)
                    .service(api_dweb_ant::v0::chunk::chunk_post)
                    .service(api_dweb_ant::v0::chunk::chunk_get)
                    .service(api_dweb_ant::v0::data::data_get)
                    .service(api_dweb_ant::v0::pointer::pointer_post)
                    .service(api_dweb_ant::v0::pointer::pointer_put)
                    .service(api_dweb_ant::v0::pointer::pointer_get)
                    .service(api_dweb_ant::v0::pointer::pointer_get_owned)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_public_post)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_public_put)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_public_get)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_public_get_owned)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_private_post)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_private_put)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_private_get)
                    .service(api_dweb_ant::v0::scratchpad::scratchpad_private_get_owned)
                    // dweb APIs
                    .service(api_dweb::v0::name::api_register_name)
                    .service(api_dweb::v0::name::api_dwebname_list)
                    .service(api_dweb::v0::app_settings::app_settings)
                    .service(api_dweb::v0::file::file_get)
                    .service(api_dweb::v0::form::data_put)
                    .service(api_dweb::v0::form::data_put_list)
                    .service(api_dweb::v0::wallet::wallet_balance_get),
            )
            .default_service(web::get().to(www::www_handler))
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api/openapi.json", api)
            })
            .app_data(Data::new(client.clone()))
            .app_data(Data::new(history_address.clone()))
            .app_data(Data::new(archive_address.clone()))
            .app_data(Data::new(directory_version_with_port.clone()))
            .app_data(Data::new(start_blocking))
            .app_data(Data::new(is_main_server))
            .into_app()
    })
    .keep_alive(Duration::from_secs(crate::services::CONNECTION_TIMEOUT));

    println!("DDDDDD HttpServer::new() OK");

    let http_server = server.bind((host, port));
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

    // if spawn_server {
    //     println!("DDDDDD spawn_server TRUE");
    //     // TODO maybe keep a map of struct {handle, port, history address, version, archive address}
    //     // TODO and provide a command to list runnning servers and addresses
    //     // TODO maybe provide a command to kill by port or port range
    //     let directory_version = match directory_version_with_port_copy2 {
    //         None => {
    //             println!("DEBUG cannot spawn serve_with_ports when provided directory_version_with_port is None");
    //             return Ok(());
    //         }
    //         Some(directory_version_with_port) => directory_version_with_port,
    //     };

    //     let server = server.bind((host.clone(), directory_version.port))?.run();
    //     println!("DDDDDD spawning....");
    //     actix_web::rt::spawn(server);
    //     println!(
    //         "Started a dweb server listening on {host}:{} for version {:?} at {:?} -> {}",
    //         directory_version.port,
    //         directory_version.version,
    //         directory_version.history_address,
    //         directory_version.archive_address
    //     );

    //     Ok(())
    // } else {
    //     println!("DDDDDD spawn_server FALSE");
    //     println!("dweb main server listening on {host}:{port}");
    //     server.bind((host, port))?.run().await
    // }
}

// This Works when called from handle_spawn()
// Panics when called from handle_dweb_open()
// -> Copy handle_spawn() next to handle_dweb_open() and gradually migrate it to find out what causes handle_dweb_open() to error
#[actix_web::main]
pub async fn simple_init_dweb_server(
    client_config: &DwebClientConfig,
    dweb_client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    spawn_server: bool,
    start_blocking: bool,
) -> Result<(), std::io::Error> {
    println!("DDDDDD simple_init_dweb_server()");
    if let Some(dweb_client) = dweb_client {
        dweb_client
    } else {
        match dweb::client::DwebClient::initialise_and_connect(&client_config).await {
            Ok(dweb_client) => dweb_client,
            Err(e) => {
                let message = format!("Failed to connect to Autonomi Network: {e}");
                println!("DEBUG: {message}");
                return Err(Error::new(NotConnected, e));
            }
        }
    };

    let port = client_config.port.unwrap_or(SERVER_PORTS_MAIN_PORT);
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

#[utoipa::path(
    responses(
        (status = StatusCode::OK,
            description = "/spawn/{port} thinks it worked!", body = str)
        ),
    tags = ["Test"],
)]
#[get("/spawn/{port}")]
async fn handle_spawn(
    _req: HttpRequest,
    port: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let port = port.into_inner().parse().unwrap_or(9999);
    let client_config = DwebClientConfig {
        port: Some(port),
        ..DwebClientConfig::default()
    };
    std::thread::spawn(move || {
        tauri_init_dweb_server(&client_config, None, None, None, false, false)
    });

    let message = format!("Spawned server on port: {port}");
    Ok(HttpResponseBuilder::new(StatusCode::OK).body(message))
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
