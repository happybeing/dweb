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

// use color_eyre::Result;
use std::io;
use std::time::Duration;

use actix_web::{
    body, dev::HttpServiceFactory, get, guard, post, web, App, HttpRequest, HttpResponse,
    HttpServer, Responder,
};
use clap::Parser;

use crate::cli_options::Opt;

const CONNECTION_TIMEOUT: u64 = 75;

pub async fn serve(host: String, port: u16) -> io::Result<()> {
    println!("dweb serve listening on {host}:{port}");
    HttpServer::new(|| {
        App::new()
            // Test routes for api-dweb.au, app-dweb.au etc
            .service(api::test::init_service("api-dweb.au"))
            .service(app::test::init_service("app-dweb.au"))
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
        "<!DOCTYPE html><head></head><body>
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
        <body>",
        request.full_url(),
        request.uri(),
        request.method(),
        request.path(),
        request.query_string(),
        request.peer_addr(),
    )
}

#[get("/awf/{datamap_address:.*}")]
async fn test_fetch_file(datamap_address: web::Path<String>) -> impl Responder {
    // return HttpResponse::Ok().body(format!(
    //     "<!DOCTYPE html><head></head><body>test /awf/&lt;DATAMAP-ADDRESS&gt;:<br/>xor: {}<body>",
    //     datamap_address.to_string()
    // ));

    // HttpResponse::Ok().body(fetdh_content(&datamap_address).await)
    HttpResponse::Ok().body("TODO: implement test_fetch_file()")
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
