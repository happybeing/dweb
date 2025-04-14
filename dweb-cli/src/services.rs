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

pub(crate) mod api_ant;
pub(crate) mod api_dweb;
pub(crate) mod helpers;
pub(crate) mod openapi;
pub(crate) mod www;

use std::io;
use std::time::Duration;

use actix_web::{dev::Service, middleware::Logger, web, web::Data, App, HttpServer};
use utoipa::OpenApi;
use utoipa_actix_web::scope::scope;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

use dweb::cache::directory_with_port::DirectoryVersionWithPort;
use dweb::client::DwebClient;

use crate::services::api_dweb::v0::name::register_builtin_names;

pub const CONNECTION_TIMEOUT: u64 = 75;

#[cfg(feature = "development")]
const DWEB_SERVICE_DEBUG: &str = "debug-dweb.au";

#[cfg(feature = "development")]
const DWEB_SERVICE_DEBUG: &str = "debug-dweb.au";

/// serve_with_ports may be called as follows:
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
pub async fn serve_with_ports(
    client: &DwebClient,
    directory_version_with_port: Option<DirectoryVersionWithPort>,
    // Host if set from the CLI.
    host: String,
    // Port when spawning the main server (ie spawn_server false). Can be set from the CLI.
    port: Option<u16>,
    // Either spawn a thread for the server and return, or do server.await
    spawn_server: bool,
    is_local_network: bool,
) -> io::Result<()> {
    register_builtin_names(is_local_network);
    let directory_version_with_port_copy = directory_version_with_port.clone();
    let directory_version_with_port = directory_version_with_port;

    // TODO control logger using CLI? (this enables Autonomi and HttpRequest logging to terminal)
    // env_logger::init_from_env(Env::default().default_filter_or("info"));

    let is_main_server = !spawn_server;
    let client = client.clone();
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                actix_cors::Cors::default()
                    .allow_any_origin()
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
            .service(
                scope(dweb::api::ANT_API_ROUTE)
                    .service(api_ant::v0::archive::post_public)
                    .service(api_ant::v0::archive::post_private)
                    .service(api_ant::v0::data::data_get)
                    .service(api_ant::v0::archive::archive_get)
                    .service(api_ant::v0::archive::get_version),
            )
            // dweb APIs
            .service(
                scope(dweb::api::DWEB_API_ROUTE)
                    .service(api_dweb::v0::name::api_register_name)
                    .service(api_dweb::v0::name::api_dwebname_list)
                    .service(api_dweb::v0::file::file_get)
                    .service(api_dweb::v0::form::data_put)
                    .service(api_dweb::v0::form::data_put_list),
            )
            .default_service(web::get().to(www::www_handler))
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api/openapi.json", api)
            })
            .app_data(Data::new(client.clone()))
            .app_data(Data::new(directory_version_with_port.clone()))
            .app_data(Data::new(is_local_network))
            .app_data(Data::new(is_main_server))
            .into_app()
    })
    .keep_alive(Duration::from_secs(crate::services::CONNECTION_TIMEOUT));

    if spawn_server {
        // TODO keep a map of struct {handle, port, history address, version, archive address}
        // TODO main server uses this to kill all spawned servers when it shuts down
        // TODO provide a command to list runnning servers and addresses
        // TODO maybe provide a command to kill by port or port range
        let directory_version = match directory_version_with_port_copy {
            None => {
                println!("DEBUG cannot spawn serve_with_ports when provided directory_version_with_port is None");
                return Ok(());
            }
            Some(directory_version_with_port_copy) => directory_version_with_port_copy,
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
        let port = match port {
            None => {
                println!("DEBUG cannot bind serve_with_ports when provided directory_version_with_port is None");
                return Ok(());
            }
            Some(port) => port,
        };
        println!("dweb main server listening on {host}:{port}");
        server.bind((host, port))?.run().await
    }
}
