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

// use actix_web::{body, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web::{
    body, dev::HttpServiceFactory, dev::ServiceRequest, dev::ServiceResponse, get, guard,
    http::header, post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};

use dweb::helpers::convert::awe_str_to_register_address;

use crate::services::request_as_html;

pub fn init_service() -> impl HttpServiceFactory {
    web::resource("/{path:.*}")
        .route(web::get().to(www_handler))
        .guard(guard::fn_guard(|ctx| {
            // println!("ctx: {ctx:?}");
            if let Some(host) = ctx.head().headers().get(header::HOST) {
                if let Ok(mut host) = &host.to_str() {
                    // println!("tesing host: {host}");
                    host = if let Some(position) = host.find(":") {
                        &host[..position]
                    } else {
                        host
                    };
                    // println!("tesing host: {host}");
                    let service_tail = String::from(".") + crate::services::DWEB_SERVICE_WWW;
                    return host.ends_with(&service_tail);
                }
            }
            false
        }))
}

/// Handle Autonomi www requests of the form:
/// fixed website:			<DIRECTORY-ADDRESS>.www-dweb.au[<PATH>][?params]
///	versioned website: 		<HISTORY-ADDRESS>.v[<version>].www-dweb.au[<PATH>][?params]
///
/// Example urls:
///     http://something.www-dweb.au:8080/here/is/a/path.html
///     http://v123.something.www-dweb.au:8080/here/is/a/history/path.html

/// WWW service - handler for Autonomi websites
pub async fn www_handler(request: HttpRequest, path: web::Path<String>) -> HttpResponse {
    println!("www_handler({path})...");
    let mut host = None;
    if let Some(req_host) = request.head().headers().get(header::HOST) {
        if let Ok(req_host) = &req_host.to_str() {
            if let Some(position) = req_host.find(":") {
                host = Some(&req_host[..position]);
            } else {
                host = Some(req_host);
            }
        }
    };

    if host.is_some() {
        let subdomains: Vec<&str> = host.unwrap().split_terminator('.').collect();
        match subdomains.len() {
            3 => {
                // <DIRECTORY-ADDRESS>.www-dweb.au has three parts
                let directory = subdomains[0];
                println!("3 -> fixed: www-dweb.au address with DIRECTORY '{directory}'");
            }
            4 => {
                // <HISTORY-ADDRESS>.v[<version>].www-dweb.au has four parts
                let version = subdomains[0];
                let history = subdomains[1];

                let version = if version.starts_with('v') {
                    match subdomains[0][1..].parse::<u64>() {
                        Ok(version) => Some(version),
                        Err(_) => None,
                    }
                } else {
                    None
                };

                let history_address = awe_str_to_register_address(history);
                println!(
                    "4 -> history www-dweb.au address with VERSION '{version:?}' and HISTORY '{history}'"
                );
            }
            _ => {
                println!("invalid www-dweb.au address");
            }
        }
    }

    let request_html = request_as_html(&request);
    let body = format!(
        "
    <!DOCTYPE html><head></head><body>
    <h3>www_handler(request, path: /{path})</h3>
    {request_html}
    </body>"
    );

    HttpResponse::Ok().body(body)
}
