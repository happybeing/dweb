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

use actix_web::{get, web, web::Data, HttpRequest, HttpResponse};

use dweb::trove::History;
use dweb::{helpers::convert::*, helpers::web::*, trove::directory_tree::DirectoryTree};

use crate::services::helpers::*;

/// Get the file metadata in a directory tree
///
/// Retrieves a PublicArchive from Autonomi and returns metadata for all files it contains.
///
/// Path parameters:
///
///     [v{version}/]{address_or_name}
///
// TODO consider changing this to return a utoipa Schema for a DirectoryTree and leave interpretation to the client
#[utoipa::path(
    responses(
        (status = 200,
            description = "The JSON representation of a DirectoryTree formatted for an SVAR file manager component.
            <p>Note: this may be changed to return a JSON representation of a DirectoryTree.", body = str)
        ),
    tags = [dweb::api::DWEB_API_ROUTE],
    params(
        ("params", description = "[v{version}/]{address_or_name}<br/><br/>Optional version (integer > 0), an address_or_name which refers to a History<DirectoryTree>"),
    )
)]
#[get("/directory-load/{params:.*}")]
pub async fn api_directory_load(
    request: HttpRequest,
    params: web::Path<String>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let params = params.into_inner();
    let decoded_params = match parse_versioned_path_params(&params) {
        Ok(params) => params,
        Err(_e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::BadRequest(),
                "/directory-load error".to_string(),
                "/directory-load invalid parameters",
            )
        }
    };

    let (version, as_name, address_or_name, remote_path) = decoded_params;
    let version = version.clone();

    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            "/directory-load error".to_string(),
            &format!("Unrecognised DWEB-NAME or invalid address: '{address_or_name}'"),
        );
    }

    let client = client.into_inner().as_ref().clone();
    let archive_address = if archive_address.is_some() {
        archive_address.unwrap()
    } else {
        let history_address = history_address.unwrap();
        let mut history = match History::<DirectoryTree>::from_history_address(
            client.clone(),
            history_address,
            false,
            0,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    "/directory-load error".to_string(),
                    "/directory-load failed to get directory History",
                )
            }
        };

        let ignore_pointer = false;
        let version = version.unwrap_or(0);
        match history
            .get_version_entry_value(version, ignore_pointer)
            .await
        {
            Ok(archive_address) => archive_address,
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::BadRequest(),
                    "/directory-load error".to_string(),
                    "/directory-load invalid parameters",
                )
            }
        }
    };

    println!(
        "DEBUG DirectoryTree::from_archive_address() with address: {}",
        archive_address.to_hex()
    );
    let directory_tree = match DirectoryTree::from_archive_address(&client, archive_address).await {
        Ok(directory_tree) => directory_tree,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/directory-load error".to_string(),
                "/directory-load failed to get directory Archive",
            )
        }
    };

    // println!(
    //     "DEBUG JSON:\n{}",
    //     json_for_svar_file_manager(&directory_tree.directory_map)
    // );

    // let remote_path = if !remote_path.is_empty() {
    //     Some(format!("/{remote_path}"))
    // } else {
    //     None
    // };

    HttpResponse::Ok().body(json_for_svar_file_manager(&directory_tree.directory_map))
}
