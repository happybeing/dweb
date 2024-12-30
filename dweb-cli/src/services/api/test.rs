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
    body, dev::HttpServiceFactory, get, guard, post, web, App, HttpRequest, HttpResponse,
    HttpServer, Responder,
};

pub fn init_service(host: &str) -> impl HttpServiceFactory {
    actix_web::web::scope("/test") // Need a guard for "api-dweb.au"
        .service(api_test)
        .guard(guard::Host(host))
}

/// Test API
/// Test url: http://api-dweb.au:8080/test/some/thing

#[get("/some/{operation}")]
pub async fn api_test(operation: web::Path<String>) -> impl Responder {
    // pub async fn api_test1() -> impl Responder {
    let body = format!("api_test() Hello, I'm api.dweb.ant/test\nThe operation is '{operation}'");
    // let body = format!("api_test1() BINGO!");

    HttpResponse::Ok().body(body)
}
