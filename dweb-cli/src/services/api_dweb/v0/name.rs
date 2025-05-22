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

use dweb::web::name::{register_name, register_name_from_string};

/// Create a short name for content on Autonomi
///
/// Register a short name (or DWEB-NAME) for a History address. The name can be used from dweb CLI or in dweb APIs until the dweb server is restarted.
///
/// Test url: http://127.0.0.1:8080/dweb-0/name-register/smart-ant/8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c
#[utoipa::path(
    responses(
        (status = StatusCode::OK,
            description = "Success", body = str)
        ),
    tags = ["Dweb"],
    params(
        ("dweb_name", description = "A short name for a content History"),
        ("history_address", description = "The hexadecimal address of a content History on Autonomi")
    ),
)]
#[get("/name-register/{dweb_name}/{history_address}")]
pub async fn api_register_name(
    request: HttpRequest,
    params: web::Path<(String, String)>,
    _client_data: Data<dweb::client::DwebClient>,
) -> impl Responder {
    println!("DEBUG api_register_name({})...", request.path().to_string());
    let (dweb_name, history_address) = params.into_inner();

    match register_name_from_string(&dweb_name, &history_address) {
        Ok(()) => HttpResponse::Ok().body("success"),
        Err(e) => HttpResponse::BadRequest().body(format!("Failed to register dweb_name - {e}")),
    }
}

use dweb::web::name::{recognised_dwebnames, RecognisedName};

/// Get the short names known to this server
///
/// List the names and corresponding History addresses known to this server
///
/// Test url: http://127.0.0.1:8080/dweb-0/name-list
#[utoipa::path(
    responses(
        (status = StatusCode::OK,
            description = "JSON list of names", body = Vec<RecognisedName>, example = json!("[{\"key\":\"awesome\",\"history_address\":\"8650c4284430522a638a6fa37dd3e8d610c65b300f89f0199a95a1a9eab0455287f8c8d137fad390654bd9f19b868a5c\"}]"))
        ),
    tags = ["Dweb"],
)]
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

// Register builtin history addresses so they can be used immediately in browser (and CLI if supported in cli_options.rs)
pub fn register_builtin_names(is_local: bool) {
    use crate::generated_rs::{builtins_local, builtins_public};

    if is_local {
        let _ = register_name_from_string("awesome", builtins_local::AWESOME_SITE_HISTORY_LOCAL);
    } else {
        let _ = register_name_from_string("awesome", builtins_public::AWESOME_SITE_HISTORY_PUBLIC);
        let _ = register_name_from_string("friends", "b1d0f2c3c1dbbd1772a40d29f664104783cc93333d3a922c5e2c17dbe07c329cee1fa4e3452329c8a5d3eeb93f9c7d80");
        // Mainnet History is at: a27b3fdb495870ace8f91005223998dc675c8e1bceb50bac66c993bb720a013c9f83d7a46e6d0daecbb3530d5249e587
        // v1 Archive: 40ea2e530a60645363ae561c8a50c165f79d8a034c4458f68f1b848c11386e45
        let _ = register_name_from_string("scratchchat", "a27b3fdb495870ace8f91005223998dc675c8e1bceb50bac66c993bb720a013c9f83d7a46e6d0daecbb3530d5249e587");
    }
}
