/*
 Copyright (c) 2025- Mark Hughes

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
    body, dev::HttpServiceFactory, get, guard, post, web, web::Data, App, HttpRequest,
    HttpResponse, HttpServer, Responder,
};

use dweb::cache::directory_version::HISTORY_NAMES;
use dweb::helpers::convert::awe_str_to_history_address;

pub fn init_service(host: &str) -> impl HttpServiceFactory {
    // TODO modify this and the get to accept /{api}/{version}/{operation} etc (see www::init_service())
    actix_web::web::scope("/dweb/v0")
        .service(api_dwebname_register)
        .guard(guard::Host(host))
}

// Test url: http://api-dweb.au:8080/dweb/v0/webname/register/smartypants/ddd

#[get("/dwebname/register/{dweb_name}/{history_address}")]
pub async fn api_dwebname_register(
    params: web::Path<(String, String)>,
    _client_data: Data<dweb::client::AutonomiClient>,
) -> impl Responder {
    let (dweb_name, history_address_string) = params.into_inner();
    match dweb::web::name::validate_dweb_name(&dweb_name) {
        Ok(()) => (),
        Err(e) => {
            return HttpResponse::BadRequest()
                .body(format!("Invalid DWEB-NAME '{dweb_name}' - {e}"));
        }
    };

    let history_address = match awe_str_to_history_address(&history_address_string) {
        Ok(history_address) => history_address,
        Err(e) => {
            return HttpResponse::BadRequest()
                .body(format!("Invalid HISTORY-ADDRESS '{dweb_name}' - {e}"));
        }
    };

    match &mut HISTORY_NAMES.lock() {
        Ok(lock) => {
            let cached_history_address = lock.get(&dweb_name);
            if cached_history_address.is_some() {
                let cached_history_address = cached_history_address.unwrap();
                if history_address != *cached_history_address {
                    return HttpResponse::BadRequest()
                        .body(format!("DWEB-NAME '{dweb_name}' already in use for HISTORY-ADDRESS '{cached_history_address}'"));
                }
                // println!("DWEB-NAME '{dweb_name}' already registered for {history_address_string}");
            } else {
                lock.insert(dweb_name.clone(), history_address);
                println!(
                    // "DWEB-NAME '{dweb_name}' successfully registered for {history_address_string}"
                );
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to access dweb name cache - {e}"));
        }
    };

    HttpResponse::Ok().body("success")
}
