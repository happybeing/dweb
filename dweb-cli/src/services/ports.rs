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

pub mod api;
mod app;
mod www;

use std::io;
use std::time::Duration;

use actix_web::{
    dev::Service, get, http::StatusCode, middleware::Logger, post, web, web::Data, App,
    HttpRequest, HttpResponse, HttpServer, Responder,
};
use clap::Parser;

use dweb::cache::directory_with_port::{self, DirectoryVersionWithPort};
use dweb::client::AutonomiClient;
use dweb::helpers::convert::str_to_xor_name;
use dweb::web::fetch::response_with_body;

use crate::cli_options::Opt;
use crate::generated_rs::register_builtin_names;

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
    client: &AutonomiClient,
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

    let client = client.clone();
    let server = HttpServer::new(move || {
        App::new()
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
            }) // <SERVICE>-dweb.au routes
            // TODO work out how to handle API and APP services without a local DNS
            // .service(api::dweb_v0::init_service(DWEB_SERVICE_API))
            // .service(app::test::init_service(DWEB_SERVICE_APP))
            //
            // <ARCHIVE-ADDRESS>|[vN].<HISTORY-ADDRESS>.www-dweb.au services must be
            // after above routes or will consume them too!
            .service(www::dweb_link::init_service())
            .service(api::dweb_v0::init_service())
            // .service(www::debug::init_service())
            //
            // TODO: (eventually!) remove these basic test routes
            // .service(hello)
            // .service(echo)
            // .service(test_fetch_file)
            // .route("/hey", web::get().to(manual_hello))
            // .route(
            //     "/test-show-request",
            //     web::get().to(manual_test_show_request),
            // )
            // .route("/test-connect", web::get().to(manual_test_connect))
            .app_data(Data::new(client.clone()))
            .app_data(Data::new(directory_version_with_port.clone()))
            .app_data(Data::new(is_local_network))
            .default_service(web::get().to(www::www::www_handler))
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
        println!("dweb serve main server listening on {host}:{port}");
        server.bind((host, port))?.run().await
    }
}

// impl Guard for HttpRequest {
//     fn check(&self, req: &GuardContext) -> bool {
//         match req.head().
//             .contains_key(http::header::CONTENT_TYPE)
//     }
// }

///////////////////////
// Earlier test routes
#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

// async fn manual_test_default_route(request: HttpRequest) -> impl Responder {
//     return HttpResponse::Ok().body(format!(
//         "<!DOCTYPE html><head></head><body>quick-test-default-route '/':<br/>uri: {}<br/>method: {}<body>",
//         request.uri(),
//         request.method()
//     ));
// }

async fn manual_test_show_request(request: HttpRequest) -> impl Responder {
    return HttpResponse::Ok().body(request_as_html(&request));
}

// Returns an HTML page detailing an HttpRequest including its headers
pub fn request_as_html(request: &HttpRequest) -> String {
    let mut headers = String::from(
        "   <tr><td></td><td></td></tr>
        <tr><td><b>HEADERS:</b></td><td></td></tr>
    ",
    );
    for (key, value) in request.headers().iter() {
        headers += format!("<tr><td>{key:?}</td><td>{value:?}</td></tr>").as_str();
    }

    format!(
        "
        <table rules='all' style='border: solid;'>
           <tr><td></td><td></td></tr>
        <tr><td><b>HttpRequest:</b></td><td></td></tr>
        <tr><td>full_url</td><td>{}</td></tr>
        <tr><td>uri</td><td>{}</td></tr>
        <tr><td>method</td><td>{}</td></tr>
        <tr><td>path</td><td>{}</td></tr>
        <tr><td>query_string</td><td>{}</td></tr>
        <tr><td>peer_addr</td><td>{:?}</td></tr>
        {headers}
        </table>
        ",
        request.full_url(),
        request.uri(),
        request.method(),
        request.path(),
        request.query_string(),
        request.peer_addr(),
    )
}

#[get("/awf/{datamap_address:.*}")]
async fn test_fetch_file(
    datamap_address: web::Path<String>,
    client_data: Data<dweb::client::AutonomiClient>,
) -> impl Responder {
    println!("test_fetch_file()...");

    let file_address = match str_to_xor_name(datamap_address.as_str()) {
        Ok(file_address) => file_address,
        Err(e) => {
            return response_with_body(
                StatusCode::BAD_REQUEST,
                Some(format!("invalid address. {e}")),
            );
        }
    };

    match client_data.data_get_public(file_address).await {
        Ok(bytes) => HttpResponse::Ok().body(bytes),
        Err(e) => {
            return response_with_body(StatusCode::NOT_FOUND, Some(format!("{e}")));
        }
    }
}

async fn manual_test_connect() -> impl Responder {
    let opt = Opt::parse();
    if let Ok(peers) = dweb::autonomi::access::network::get_peers(opt.peers).await {
        if let Ok(_client) = dweb::autonomi::actions::connect_to_network(peers).await {
            return HttpResponse::Ok().body(
                "Testing connect to Autonomi..\
           SUCCESS!",
            );
        } else {
            return HttpResponse::Ok().body(
                "Testing connect to Autonomi..\
           ERROR: failed to connect",
            );
        };
    } else {
        return HttpResponse::Ok().body(
            "Testing connect to Autonomi..\
           ERROR: failed to get peers",
        );
    };
}
