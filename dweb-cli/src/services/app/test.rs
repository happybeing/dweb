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

use actix_web::{
    body, dev::HttpServiceFactory, get, guard, post, web, App, HttpRequest, HttpResponse,
    HttpServer, Responder,
};

pub fn init_service(host: &str) -> impl HttpServiceFactory {
    actix_web::web::scope("/test") // Need a guard for "api-dweb.au"
        .service(test_app)
        .guard(guard::Host(host))
}

/// Example app
/// Test with url: http://app-dweb.au:8080/test/
#[get("/")]
pub async fn test_app() -> impl Responder {
    let script = String::from(
        "
        console.log('test_app() script...');
        var xhr = new XMLHttpRequest();
        xhr.open('GET', '/hey', true);
        xhr.onload = function() {
        if (xhr.status === 200) {
            console.log('response from /hey:' + xhr.responseText);
        }
        };
        xhr.send();",
    );

    let body = format!(
        "<!DOCTYPE html>
        <head>
        </head>
        <script>
        {script}
        </script>
        <body>
        Testing dweb.ant/test-app
        <body>"
    );
    HttpResponse::Ok().body(body)
}
