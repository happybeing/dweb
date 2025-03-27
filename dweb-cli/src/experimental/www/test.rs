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
use actix_web::{dev::HttpServiceFactory, guard, http::header, web, HttpRequest, HttpResponse};

use dweb::helpers::web::request_as_html;

pub fn init_service() -> impl HttpServiceFactory {
    web::resource("/test/{path:.*}")
        .route(web::get().to(www_test))
        .guard(guard::fn_guard(|ctx| {
            // println!("ctx: {ctx:?}");
            if let Some(host) = ctx.head().headers().get(header::HOST) {
                if let Ok(mut host) = &host.to_str() {
                    println!("tesing host: {host}");
                    host = if let Some(position) = host.find(":") {
                        &host[..position]
                    } else {
                        host
                    };
                    println!("tesing host: {host}");
                    let service_tail = String::from(".") + dweb::web::DWEB_SERVICE_WWW;
                    return host.ends_with(&service_tail);
                }
            }
            false
        }))
}

/// Test WWW
/// Test url: http://www-dweb.au:8080/here/is/a/path.html

// #[get("/")]
pub async fn www_test(request: HttpRequest, path: web::Path<String>) -> HttpResponse {
    println!("www_test()...");
    let request_html = request_as_html(&request);
    let body = format!(
        "
    <!DOCTYPE html><head></head><body>
    <h3>www_test(request, path: /{path})</h3>
    {request_html}
    </body>"
    );

    HttpResponse::Ok().body(body)
}
