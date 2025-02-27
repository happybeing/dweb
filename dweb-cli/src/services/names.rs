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

mod api;
mod app;
mod www;

// use color_eyre::Result;
use std::io;
use std::time::Duration;

use crate::cli_options::Opt;
use actix_web::{
    dev::Service, get, http::StatusCode, middleware::Logger, post, web, web::Data, App,
    HttpRequest, HttpResponse, HttpServer, Responder,
};
use clap::Parser;

use dweb::client::AutonomiClient;
use dweb::helpers::convert::str_to_xor_name;
use dweb::web::fetch::response_with_body;

use crate::generated_rs::register_builtin_names;
use crate::services::{CONNECTION_TIMEOUT, DWEB_SERVICE_API, DWEB_SERVICE_APP};

#[cfg(feature = "development")]
const DWEB_SERVICE_DEBUG: &str = "debug-dweb.au";

pub async fn serve_with_names(
    client: AutonomiClient,
    host: String,
    port: u16,
    is_local_network: bool,
) -> io::Result<()> {
    register_builtin_names(is_local_network);
    // TODO control using CLI? (this enables Autonomi and HttpRequest logging to terminal)
    // env_logger::init_from_env(Env::default().default_filter_or("info"));

    println!(
        "Starting a dweb server for names (which requires a local DNS), listening on {host}:{port}"
    );
    HttpServer::new(move || {
        App::new()
            // Macro logging using env_logger for both actix and libs such as Autonomi
            .wrap(Logger::default())
            // Log Requests and Responses to terminal
            .wrap_fn(|req, srv| {
                println!("DEBUG HttpRequest : {} {}", req.head().method, req.path());
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
                    println!("DEBUG HttpResponse: {} {}", res.status(), reason);

                    Ok(res)
                }
            }) // <SERVICE>-dweb.au routes
            // TODO add routes for SERVICE: solid, rclone etc.
            .service(api::dweb_v0::init_service(DWEB_SERVICE_API))
            .service(app::test::init_service(DWEB_SERVICE_APP))
            //
            // <ARCHIVE-ADDRESS>|[vN].<HISTORY-ADDRESS>.www-dweb.au services must be
            // after above routes or will consume them too!
            .service(www::test::init_service())
            .service(www::www::init_service())
            .service(www::debug::init_service())
            //
            // TODO: (eventually!) remove these basic test routes
            .service(hello)
            .service(echo)
            .service(test_fetch_file)
            .route("/hey", web::get().to(manual_hello))
            .route(
                "/test-show-request",
                web::get().to(manual_test_show_request),
            )
            .route("/test-connect", web::get().to(manual_test_connect))
            .app_data(Data::new(client.clone()))
            .default_service(web::get().to(manual_test_default_route))
    })
    .keep_alive(Duration::from_secs(CONNECTION_TIMEOUT))
    .bind((host.as_str(), port))?
    .run()
    .await
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

async fn manual_test_default_route(request: HttpRequest) -> impl Responder {
    return HttpResponse::Ok().body(format!(
        "<!DOCTYPE html><head></head><body>test-default-route '/':<br/>uri: {}<br/>method: {}<body>",
        request.uri(),
        request.method()
    ));
}

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
