/*
 Copyright (c) 2024-2025 Mark Hughes

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
mod app;

pub(crate) mod api_dweb;
pub(crate) mod api_dweb_ant;
pub(crate) mod helpers;
pub mod openapi;
pub(crate) mod www;

use std::io;
use std::io::Error;
use std::io::ErrorKind::NotConnected;
use std::time::Duration;

use actix_web::dev::HttpServiceFactory;
use actix_web::{dev::Service, middleware::Logger, web, web::Data, App, HttpServer};
use utoipa::OpenApi;
use utoipa_actix_web::scope::scope;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

use crate::StopHandle;
use dweb::cache::directory_with_port::DirectoryVersionWithPort;
use dweb::client::{DwebClient, DwebClientConfig};
use dweb::web::SERVER_PORTS_MAIN_PORT;

pub const CONNECTION_TIMEOUT: u64 = 75;

#[cfg(feature = "development")]
pub const DWEB_SERVICE_DEBUG: &str = "debug-dweb.au";

/// init_dweb_service() may be called as follows:
///
/// DDDDDD TODO move to lib.rs
/// DDDDDD TODO review this...
///
/// Note: The presence of DirectoryVersionWithPort indicates a server on the port for a directory/website.
///
/// Via CLI 'dweb serve': start (NOT spawn) the main 'with ports' server on the supplied port with
///     DirectoryVersionsWithPort as None this server stays alive until killed on the command line. Its job is to:
///       1) respond to /dweb-open URLs (e.g. when opened by 'dweb open') by looking up the directory
///     version and if no server is running, call serve_with_ports() to start one before redirecting the link;
///       2) manage DirectoryVersionsWithPort servers by killing them when it shuts down and supporting a web API
///     for listing and killing other DirectoryVersionsWithPort servers.
///
/// Via dweb open: when it uses the server API to open an Autonomi link on the main server port and no DirectoryVersionWithPort
///     has been found.
///
/// Via any URL handler of a /dweb-open URL, and behave as above to look for a server and if no DirectoryVersionsWithPort
///     is found, call serve_with_ports() to spawn a new one. Then redirect the link.
///
// Tweaking for:
// 1. Simpler flow: leave spawn and blocking outside to be controlled by DwebService::start()
#[actix_web::main]
pub async fn init_dweb_server(
    client_config: &DwebClientConfig,
    dweb_client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    // DDDDDD TODO remove these...
    spawn_server: bool,
    start_blocking: bool,
) -> io::Result<()> {
    println!("DDDDDD #[actix_web::main] init_dweb_server() calling init_dweb_server_blocking()");
    init_dweb_server_blocking(
        client_config,
        dweb_client,
        stop_handle,
        directory_version_with_port,
        spawn_server,
        start_blocking,
    )
    .await;
    Ok(())
}

pub async fn init_dweb_server_blocking(
    client_config: &DwebClientConfig,
    dweb_client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    // DDDDDD TODO remove these...
    spawn_server: bool,
    start_blocking: bool,
) -> Result<(), std::io::Error> {
    println!("DDDDDD V1 init_dweb_server()...");
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

    let is_main_server = directory_version_with_port.is_none();
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

    let http_server = server.bind((host.clone(), port));
    match http_server {
        Ok(server) => {
            match directory_version_with_port_copy2 {
                None => {
                    println!("Started the main dweb server listening on {host}:{port}");
                }
                Some(directory_version) => {
                    println!(
                    "Started site/app dweb server listening on {host}:{} for version {:?} at {:?} -> {}",
                    directory_version.port,
                    directory_version.version,
                    directory_version.history_address,
                    directory_version.archive_address
                );
                }
            };
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

#[actix_web::main]
pub async fn init_dweb_server_old(
    client_config: &DwebClientConfig,
    client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    spawn_server: bool,
) -> io::Result<()> {
    println!("#[actix_web::main] init_dweb_server() calling init_dweb_server_blocking()");
    init_dweb_server_blocking_old(
        client_config,
        client,
        stop_handle,
        directory_version_with_port,
        spawn_server,
        false,
    );
    Ok(())
}

pub async fn init_dweb_server_blocking_old(
    client_config: &DwebClientConfig,
    client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    spawn_server: bool,
    start_blocking: bool,
) -> io::Result<()> {
    println!("init_dweb_server_blocking()...");
    let client = if client.is_some() {
        client.unwrap()
    } else {
        match dweb::client::DwebClient::initialise_and_connect(&client_config).await {
            Ok(client) => client,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to connect to Autonomi Network",
                ))
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
    let port = client.port;
    let client = client.clone();
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
            .service(www::dweb_open::dweb_open)
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

    if spawn_server {
        // TODO maybe keep a map of struct {handle, port, history address, version, archive address}
        // TODO and provide a command to list runnning servers and addresses
        // TODO maybe provide a command to kill by port or port range
        let directory_version = match directory_version_with_port_copy2 {
            None => {
                println!("DEBUG cannot spawn serve_with_ports when provided directory_version_with_port is None");
                return Ok(());
            }
            Some(directory_version_with_port) => directory_version_with_port,
        };

        let server = server.bind((host.clone(), directory_version.port))?.run();
        actix_web::rt::spawn(server);
        println!(
            "Started a dweb server listening on {host}:{} for version {:?} at {:?} -> {}",
            directory_version.port,
            directory_version.version,
            directory_version.history_address,
            directory_version.archive_address
        );

        Ok(())
    } else {
        println!("dweb main server listening on {host}:{port}");
        server.bind((host, port))?.run().await
    }
}

/*
#[actix_web::main]
async fn init_dweb_server_interim(
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

use actix_web::{
    dev::ServerHandle, get, http::StatusCode, web::Query, HttpRequest, HttpResponse,
    HttpResponseBuilder, Result,
};
// use apicize_lib::{
//     oauth2_pkce::{generate_authorization, refresh_token, retrieve_access_token},
//     PkceTokenResult,
// };
// use tauri::{AppHandle, Emitter, Manager, Url};

#[get("/")]
async fn handle_get(_req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponseBuilder::new(StatusCode::OK).body("Bingo!"))
}

#[get("/spawn/{port}")]
async fn handle_spawn(
    _req: HttpRequest,
    port: web::Path<String>,
    app_handle: Data<AppHandle>,
) -> Result<HttpResponse, actix_web::Error> {
    let port = port.into_inner().parse().unwrap_or(9999);
    let app_handle = app_handle.into_inner().app_handle().clone();
    let server = Some(std::thread::spawn(move || {
        init_dweb_server(app_handle, port, None)
    }));

    let message = format!("Spawed server on port: {port} ...NOT!");
    Ok(HttpResponseBuilder::new(StatusCode::OK).body(message))
}
 */

#[actix_web::main]
pub async fn old_serve_with_ports(
    client: &DwebClient,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Either spawn a thread for the server and return, or do server.await
    spawn_server: bool,
    is_local_network: bool,
) -> io::Result<()> {
    // register_builtin_names(is_local_network);
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
    let port = client.port;
    let client = client.clone();
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
            .service(www::dweb_open::dweb_open)
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
            .app_data(Data::new(is_main_server))
            .into_app()
    })
    .keep_alive(Duration::from_secs(crate::services::CONNECTION_TIMEOUT));

    if spawn_server {
        // TODO maybe keep a map of struct {handle, port, history address, version, archive address}
        // TODO and provide a command to list runnning servers and addresses
        // TODO maybe provide a command to kill by port or port range
        let directory_version = match directory_version_with_port_copy2 {
            None => {
                println!("DEBUG cannot spawn serve_with_ports when provided directory_version_with_port is None");
                return Ok(());
            }
            Some(directory_version_with_port) => directory_version_with_port,
        };

        let server = server.bind((host.clone(), directory_version.port))?.run();
        actix_web::rt::spawn(server);
        println!(
            "Started a dweb server listening on {host}:{} for version {:?} at {:?} -> {}",
            directory_version.port,
            directory_version.version,
            directory_version.history_address,
            directory_version.archive_address
        );

        Ok(())
    } else {
        println!("dweb main server listening on {host}:{port}");
        server.bind((host, port))?.run().await
    }
}
