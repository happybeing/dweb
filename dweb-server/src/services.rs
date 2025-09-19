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

/// init_dweb_service_blocking() and init_dweb_service_non_blocking()
///
/// Two init functions are provided below to allow a server to be started as blocking or non-blocking.
///
/// Note: Some(directory_version_with_port) indicates a server on the port for a directory/website.
///
/// Via CLI 'dweb serve': start (NOT spawn) the main 'with ports' server on the supplied port with
///     directory_version_with_port as None this server stays alive until killed on the command line. Its job is to:
///       1) respond to /dweb-open URLs (e.g. when opened by 'dweb open') by looking up the directory
///     version and if no server is running, call init_dweb_server_non_blocking() to start one before redirecting the link;
///       2) manage DirectoryVersionsWithPort servers by killing them when it shuts down and supporting a web API
///     for listing and killing other DirectoryVersionsWithPort servers.
///
/// Via dweb open: when it uses the server API to open an Autonomi link on the main server port and no DirectoryVersionWithPort
///     has been found.
///
/// Via any URL handler of a /dweb-open URL, and behave as above to look for a server and if no DirectoryVersionsWithPort
///     is found, call init_dweb_server_non_blocking() to spawn a new one. Then redirect the link.
///
/// See DwebService::start() for example usage.

#[actix_web::main]
pub async fn init_dweb_server_non_blocking(
    client_config: &DwebClientConfig,
    dweb_client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
) -> io::Result<()> {
    let _ = init_dweb_server_blocking(
        client_config,
        dweb_client,
        stop_handle,
        directory_version_with_port,
    )
    .await;
    Ok(())
}

pub async fn init_dweb_server_blocking(
    client_config: &DwebClientConfig,
    dweb_client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
) -> Result<(), std::io::Error> {
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

    // Determine number of Actix workers from env var DWEB_WORKERS (default 12)
    let workers = get_worker_count_from_env();

    let http_server = HttpServer::new(move || {
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
            .app_data(Data::new(is_main_server))
            .into_app()
    })
    .keep_alive(Duration::from_secs(crate::services::CONNECTION_TIMEOUT))
    .workers(workers)
    .bind((host.clone(), port));
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
}

const DEFAULT_WORKERS: usize = 12;

fn parse_workers(value: Option<String>) -> usize {
    match value.and_then(|s| s.trim().parse::<usize>().ok()) {
        Some(n) if n >= 1 => n,
        Some(_) => 1, // clamp to minimum of 1
        None => DEFAULT_WORKERS,
    }
}

fn get_worker_count_from_env() -> usize {
    parse_workers(std::env::var("DWEB_WORKERS").ok())
}