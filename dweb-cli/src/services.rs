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
use std::sync::Arc;

use actix_web::{dev::Service, middleware::Logger, web, web::Data, App, HttpServer};
use utoipa::OpenApi;
use utoipa_actix_web::scope::scope;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;
use rustls::{ServerConfig, pki_types::{CertificateDer, PrivateKeyDer}};
use rcgen::{generate_simple_self_signed, CertifiedKey};

use dweb::cache::directory_with_port::DirectoryVersionWithPort;
use dweb::client::DwebClient;

use crate::services::api_dweb::v0::name::register_builtin_names;

pub const CONNECTION_TIMEOUT: u64 = 75;

#[cfg(feature = "development")]
const DWEB_SERVICE_DEBUG: &str = "debug-dweb.au";

/// Generate a self-signed certificate for HTTPS
fn generate_self_signed_cert() -> std::io::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to generate certificate: {}", e)))?;

    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    let cert_chain = vec![CertificateDer::from(cert_der)];
    let private_key = PrivateKeyDer::try_from(key_der)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse private key: {}", e)))?;

    Ok((cert_chain, private_key))
}

/// Create rustls ServerConfig with self-signed certificate
fn create_rustls_config() -> std::io::Result<ServerConfig> {
    // Install the default crypto provider if not already installed
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    
    let (cert_chain, private_key) = generate_self_signed_cert()?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create TLS config: {}", e)))?;

    Ok(config)
}

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
    // Enable HTTPS with self-signed certificate
    https: bool,
) -> io::Result<()> {
    register_builtin_names(is_local_network);
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
            .service(
                scope(dweb::api::ANT_API_ROUTE)
                    .service(api_ant::v0::archive::archive_post_public)
                    .service(api_ant::v0::archive::archive_post_private)
                    .service(api_ant::v0::archive::archive_get)
                    .service(api_ant::v0::archive::archive_get_version)
                    .service(api_ant::v0::chunk::chunk_post)
                    .service(api_ant::v0::chunk::chunk_get)
                    .service(api_ant::v0::pointer::pointer_post)
                    .service(api_ant::v0::pointer::pointer_put)
                    .service(api_ant::v0::pointer::pointer_get)
                    .service(api_ant::v0::pointer::pointer_get_owned)
                    .service(api_ant::v0::scratchpad::scratchpad_public_post)
                    .service(api_ant::v0::scratchpad::scratchpad_public_put)
                    .service(api_ant::v0::scratchpad::scratchpad_public_get)
                    .service(api_ant::v0::scratchpad::scratchpad_public_get_owned)
                    .service(api_ant::v0::scratchpad::scratchpad_private_post)
                    .service(api_ant::v0::scratchpad::scratchpad_private_put)
                    .service(api_ant::v0::scratchpad::scratchpad_private_get)
                    .service(api_ant::v0::scratchpad::scratchpad_private_get_owned)
                    .service(api_ant::v0::data::data_get),
            )
            // dweb APIs
            .service(
                scope(dweb::api::DWEB_API_ROUTE)
                    .service(api_dweb::v0::name::api_register_name)
                    .service(api_dweb::v0::name::api_dwebname_list)
                    .service(api_dweb::v0::file::file_get)
                    .service(api_dweb::v0::form::data_put)
                    .service(api_dweb::v0::form::data_put_list)
                    .service(api_dweb::v0::app_settings::app_settings),
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

        let server_future = if https {
            let config = create_rustls_config()?;
            server.bind_rustls_0_23((host.clone(), directory_version.port), config)?.run()
        } else {
            server.bind((host.clone(), directory_version.port))?.run()
        };
        
        actix_web::rt::spawn(server_future);
        let protocol = if https { "https" } else { "http" };
        println!(
            "Started a dweb server listening on {protocol}://{host}:{} for version {:?} at {:?} -> {}",
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
        
        let protocol = if https { "https" } else { "http" };
        println!("dweb main server listening on {protocol}://{host}:{port}");
        
        if https {
            let config = create_rustls_config()?;
            server.bind_rustls_0_23((host, port), config)?.run().await
        } else {
            server.bind((host, port))?.run().await
        }
    }
}
