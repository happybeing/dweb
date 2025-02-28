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
    body,
    dev::{HttpServiceFactory, ServiceRequest, ServiceResponse},
    get, guard,
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    post,
    web::{self, Data},
    App, Error, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use mime;

use dweb::web::fetch::{fetch_website_version, response_redirect, response_with_body};
use dweb::web::DWEB_SERVICE_WWW;

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
                    let service_tail = String::from(".") + DWEB_SERVICE_WWW;
                    return host.ends_with(&service_tail);
                }
            }
            false
        }))
}

/// Handle Autonomi www requests of the form:
///     http://something.www-dweb.au:8080/here/is/a/path.html
///     http://v123.something.www-dweb.au:8080/here/is/a/history/path.html
pub async fn www_handler(
    request: HttpRequest,
    path: web::Path<String>,
    client: Data<dweb::client::AutonomiClient>,
) -> HttpResponse {
    println!("DEBUG www_handler(/{path})...");
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

    if !host.is_some() {
        return response_with_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(String::from("www_handler() failed to determine host")),
        );
    };

    let dweb_host = match dweb::web::name::decode_dweb_host(host.unwrap()) {
        Ok(dweb_host) => dweb_host,
        Err(e) => return response_with_body(StatusCode::BAD_REQUEST, Some(format!("{e}"))),
    };

    let (version, directory_version) = match fetch_website_version(&client, &dweb_host).await {
        Ok(directory_version) => directory_version,
        Err(e) => {
            return response_with_body(
                StatusCode::BAD_REQUEST,
                Some(format!("fetch_website_version() error: {e}")),
            )
        }
    };

    let directory_tree = if !directory_version.directory_tree.is_some() {
        return response_with_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(
                "fetch_website_version() returned invalid directory_version - this appears to be a bug".to_string(),
            ),
        );
    } else {
        directory_version.directory_tree.unwrap()
    };

    // If the dweb_host is not versioned, generate a versioned DwebHost URL and redirect to that
    // It will be in the cache so no extra network access is incurred, just an extra response/request with the browser
    println!(
        "DEBUG version: {}, dweb_host.version: {:?}",
        version, dweb_host.version
    );
    if version != 0 && dweb_host.version.is_none() {
        let versioned_host = format!("v{version}.{}.{}", &dweb_host.dweb_name, DWEB_SERVICE_WWW);
        let path = String::from(path.as_str());
        return response_redirect(&request, &versioned_host, None, Some(path));
    }

    match directory_tree.lookup_web_resource(&(String::from("/") + path.as_str())) {
        Ok((file_address, content_type)) => match client.data_get_public(file_address).await {
            Ok(content) => {
                let mut response = HttpResponse::Ok();
                if let Some(content_type) = content_type {
                    response.insert_header(("Content-Type", content_type.as_str()));
                }
                return response.body(content);
            }
            Err(e) => {
                return response_with_body(
                    StatusCode::BAD_GATEWAY,
                    Some(String::from(format!(
                        "DirectoryTree::lookup_web_resource({}) failed: {e}",
                        path.as_str()
                    ))),
                );
            }
        },
        Err(e) => {
            let status_code = if let Ok(status_code) = StatusCode::from_u16(e.as_u16()) {
                status_code
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            return response_with_body(
                status_code,
                Some(String::from(format!(
                    "DirectoryTree::lookup_web_resource({}) failed",
                    path.as_str()
                ))),
            );
        }
    };
}
