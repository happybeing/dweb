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
pub(crate) mod openapi;
pub(crate) mod www;

use std::time::Duration;

use actix_web::{dev::Service, middleware::Logger, web, web::Data, App, HttpServer};
use color_eyre::eyre::eyre;
use utoipa::OpenApi;
use utoipa_actix_web::scope::scope;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

use crate::StopHandle;
use dweb::client::{DwebClient, DwebClientConfig};

pub const CONNECTION_TIMEOUT: u64 = 75;

/// init_dweb_server - cut down vertions of serve_with_ports for dweb-server PoC
///
/// TODO dweb-server: move serve_with_ports here in full and rename file as server.rs
#[actix_web::main]
pub async fn init_dweb_server(
    port: u16,
    client_config: &DwebClientConfig,
    client: Option<DwebClient>,
    stop_handle: Option<Data<StopHandle>>,
) -> Result<(), std::io::Error> {
    let spawn_server = false;
    let is_main_server = !spawn_server;
    let client = if client.is_some() {
        client.unwrap()
    } else {
        let mut client_config = client_config.clone();
        client_config.port = Some(port);
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

    let host = client.host.clone();
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
            .service(
                scope(dweb::api::DWEB_API_ROUTE)
                    // dweb Enhanced Automonomi APIs
                    .service(api_dweb_ant::v0::data::data_get),
            )
            .service(handle_spawn)
            .default_service(web::get().to(www::www_handler))
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api/openapi.json", api)
            })
            .app_data(Data::new(client.clone()))
            .app_data(Data::new(is_main_server))
            .into_app()
    })
    .keep_alive(Duration::from_secs(CONNECTION_TIMEOUT));

    println!("dweb main server listening on {host}:{port}");
    server.bind((host, port))?.run().await
}

// Test ability to spawn a server within a handler
use actix_web::{get, http::StatusCode, HttpRequest, HttpResponse, HttpResponseBuilder, Result};

//  TODO remove from demo (this is just testing the approach works)
/// Test handler can spawn a server
#[utoipa::path(
    tags = ["Test"],
    params(
        ("port", description = "the port number you want the spawned server to listen on"),
    )
)]
#[get("/spawn/{port}")]
async fn handle_spawn(
    _req: HttpRequest,
    port: actix_web::web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let port = port.into_inner().parse().unwrap_or(9999);

    let mut client_config = client.client_config.clone();
    client_config.port = Some(port);
    std::thread::spawn(move || {
        init_dweb_server(
            port,
            &client_config,
            Some(client.into_inner().as_ref().clone()),
            None,
        )
    });

    let message = format!("Spawed server on port: {port}");
    Ok(HttpResponseBuilder::new(StatusCode::OK).body(message))
}
