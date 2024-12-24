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

pub mod serve_aw;

// use color_eyre::Result;
use std::io;
use std::time::Duration;

use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::Parser;

const CONNECTION_TIMEOUT: u64 = 75;

use crate::cli_options::Opt;

// A localhost server providing dweb APIs for a browser and other local apps
pub(crate) async fn serve(port: u16) -> io::Result<()> {
    let host = dweb::helpers::convert::LOCALHOST;

    println!("starting dweb server at: http://{host}:{port}");
    HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(echo)
            .service(test_fetch_file)
            .route("/hey", web::get().to(manual_hello))
            .route(
                "/test-show-request",
                web::get().to(manual_test_show_request),
            )
            .route("/test-connect", web::get().to(manual_test_connect))
            // .service(web::scope("/awf").default_service(web::get().to(manual_test_default_route)))
            .default_service(web::get().to(manual_test_default_route))
    })
    .keep_alive(Duration::from_secs(CONNECTION_TIMEOUT))
    .bind((host, port))?
    .run()
    .await
}

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
    return HttpResponse::Ok().body(format!(
        "<!DOCTYPE html><head></head><body>test-show-request:<br/>uri: {}<br/>method: {}<body>",
        request.uri(),
        request.method()
    ));
}

#[get("/awf/{datamap_address:.*}")]
async fn test_fetch_file(datamap_address: web::Path<String>) -> impl Responder {
    // return HttpResponse::Ok().body(format!(
    //     "<!DOCTYPE html><head></head><body>test /awf/&lt;DATAMAP-ADDRESS&gt;:<br/>xor: {}<body>",
    //     datamap_address.to_string()
    // ));

    // HttpResponse::Ok().body(fetch_content(&datamap_address).await)
    HttpResponse::Ok().body("TODO: implement test_fetch_file()")
}

async fn manual_test_connect() -> impl Responder {
    // TODO FIX: need a static peers: PeersArgs, when connecting (access insite the helper connect::connect_to_network())
    // TODO LATER try maybe creating a client that connects in main and is re-used via a static?
    if let Ok(_client) = crate::connect::connect_to_network().await {
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
}
