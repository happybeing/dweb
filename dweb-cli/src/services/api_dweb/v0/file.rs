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

use actix_web::{
    get,
    http::{header, StatusCode},
    web,
    web::Data,
    HttpRequest, HttpResponse, HttpResponseBuilder,
};

use dweb::trove::History;
use dweb::{helpers::convert::*, trove::directory_tree::DirectoryTree};

use crate::services::helpers::*;

/// Get a file from a content History or directory on the network
///
#[utoipa::path(
    responses(
        (status = 200)
        ),
    tags = ["Dweb"],
    params(
        ("address_or_name", description = "The hexadecimal address or short name of a content History, or the address of a directory, on Autonomi"),
        ("file_path", description = "The full path of a file in the referenced directory"),
    ),
)]
#[get("/file/{address_or_name}/{file_path:.*}")]
pub async fn file_get(
    request: HttpRequest,
    params: web::Path<(String, String)>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let (address_or_name, file_path) = params.into_inner();
    let (history_address, archive_address) = address_tuple_from_address_or_name(&address_or_name);
    if history_address.is_none() && archive_address.is_none() {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            "/file error".to_string(),
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
                    "/file error".to_string(),
                    &format!("/file failed to get directory History - {e}"),
                )
            }
        };

        let ignore_pointer = false;
        match history.get_version_entry_value(0, ignore_pointer).await {
            Ok(archive_address) => archive_address,
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::BadRequest(),
                    "/file error".to_string(),
                    &format!("/file directory History failed to get most recent version - {e}"),
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
                "/file error".to_string(),
                &format!("/file failed to get directory Archive - {e}"),
            )
        }
    };

    let (data_address, content_type) = match directory_tree.lookup_file(&file_path, false) {
        Ok((data_address, content_type)) => (data_address, content_type),
        Err(_e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/file error".to_string(),
                "/file file not found in directory",
            )
        }
    };

    let content_type = if content_type.is_some() {
        content_type.unwrap().clone()
    } else {
        String::from("text/plain")
    };

    let content = match client.data_get_public(data_address).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                "/file error".to_string(),
                &format!("/file failed to get file from network - {e}"),
            );
        }
    };

    HttpResponseBuilder::new(StatusCode::OK)
        .insert_header((header::CONTENT_TYPE, content_type.as_str()))
        .body(content)
}
