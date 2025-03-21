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

use actix_web::{get, web, web::Data, HttpRequest, HttpResponse, Responder};

use dweb::cache::directory_with_name::HISTORY_NAMES;
use dweb::helpers::convert::str_to_history_address;

/// Register a DWEB-NAME and optionally redirect to the Dweb URL for the most recent version
/// Optional query parameters control the redirection:
///
/// Test url: http://127.0.0.1:8080/dweb/v0/name_register/smart-ant/91ab27dd1dc342f36c9f16fbe4ea725372d46a857677299d0336bb5eff24392da5d4412c36b6925a4b1857cc558f31e4ef4aae8c3170a4e3d6251bbb637a313d31b5b887aa20a3c81fc358981ccf9d19
#[get("/name-register/{dweb_name}/{history_address}")]
pub async fn api_dwebname_register(
    request: HttpRequest,
    params: web::Path<(String, String)>,
    _client_data: Data<dweb::client::DwebClient>,
) -> impl Responder {
    println!(
        "DEBUG api_dwebname_register({})...",
        request.path().to_string()
    );
    let (dweb_name, history_address_string) = params.into_inner();

    // let qs = QString::from(req.query_string());
    // let redirect: bool = match qs.get("redirect").unwrap_or("true") {
    //     "false" => false,
    //     "0" => false,
    //     _ => true,
    // };

    match dweb::web::name::validate_dweb_name(&dweb_name) {
        Ok(_) => (),
        Err(e) => {
            return HttpResponse::BadRequest()
                .body(format!("Invalid DWEB-NAME '{dweb_name}' - {e}"));
        }
    };

    let history_address = match str_to_history_address(&history_address_string) {
        Ok(history_address) => history_address,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!(
                "Invalid HISTORY-ADDRESS '{history_address_string}' - {e}"
            ));
        }
    };

    match &mut HISTORY_NAMES.lock() {
        Ok(lock) => {
            let cached_history_address = lock.get(&dweb_name);
            if cached_history_address.is_some() {
                let cached_history_address = cached_history_address.unwrap();
                if history_address != *cached_history_address {
                    return HttpResponse::BadRequest().body(format!(
                        "DWEB-NAME '{dweb_name}' already in use for HISTORY-ADDRESS '{}'",
                        cached_history_address.to_hex()
                    ));
                }
                println!("DWEB-NAME '{dweb_name}' already registered for {history_address_string}");
            } else {
                lock.insert(dweb_name.clone(), history_address);
                println!(
                    "DWEB-NAME '{dweb_name}' successfully registered for {history_address_string}"
                );
            }
            // if redirect {
            //     println!("DEBUG redirecting...");
            //     return response_redirect(
            //         &req,
            //         &(dweb_name.clone() + "." + DWEB_SERVICE_WWW),   needs to redirect to port
            //         None,
            //         None,
            //     );
            // };
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to access dweb name cache - {e}"));
        }
    };

    HttpResponse::Ok().body("success")
}

use dweb::web::name::recognised_dwebnames;

/// TODO add documentation
#[get("/name-list")]
pub async fn api_dwebname_list() -> impl Responder {
    println!("DEBUG api_dwebname_list(()...");
    let names_vec = match recognised_dwebnames() {
        Ok(names_vec) => names_vec,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to gather names - {e}"));
        }
    };

    let body = match serde_json::to_string(&names_vec) {
        Ok(json_string) => json_string,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to serialise names list - {e}"));
        }
    };

    HttpResponse::Ok().body(body)
}
