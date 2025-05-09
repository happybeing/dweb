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

use crate::services::helpers::*;
use dweb::files::directory::{get_content_using_hex, Tree};
use dweb::helpers::convert::*;
use dweb::history::History;

/// Get a file from a content History or directory on the network
///
#[utoipa::path(
    responses(
        (status = StatusCode::OK)
        ),
    tags = ["Dweb"],
    params(
        ("address_or_name", description = "The hexadecimal address or short name of a content History, or the address of a directory, on Autonomi"),
        ("file_path", description = "The full path of a file in the referenced directory"),
    ),
)]
#[get("/file/{datamap_address_or_name}/{file_path:.*}")]
pub async fn file_get(
    request: HttpRequest,
    params: web::Path<(String, String)>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/file/{datamap_address_or_name}/{file_path:.*} GET";
    let _rest_handler = "file_get()";

    let (datamap_address_or_name, file_path) = params.into_inner();
    let (datamap_chunk, history_address, mut archive_address) =
        tuple_from_datamap_address_or_name(&datamap_address_or_name);
    if datamap_chunk.is_none() && history_address.is_none() && archive_address.is_none() {
        return make_error_response_page(
            None,
            &mut HttpResponse::BadRequest(),
            rest_operation.to_string(),
            &format!("Unrecognised DWEB-NAME or invalid datamap chunk or data address: '{datamap_address_or_name}'"),
        );
    }

    let client = client.into_inner().as_ref().clone();
    archive_address = if history_address.is_some() {
        let history_address = history_address.unwrap();
        let mut history =
            match History::<Tree>::from_history_address(client.clone(), history_address, false, 0)
                .await
            {
                Ok(history) => history,
                Err(e) => {
                    return make_error_response_page(
                        None,
                        &mut HttpResponse::NotFound(),
                        rest_operation.to_string(),
                        &format!("/file failed to get directory History - {e}"),
                    )
                }
            };

        let ignore_pointer = false;
        match history.get_version_entry_value(0, ignore_pointer).await {
            Ok(archive_address) => Some(archive_address),
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::BadRequest(),
                    rest_operation.to_string(),
                    &format!("{rest_operation} History failed to get most recent version - {e}"),
                );
            }
        }
    } else {
        archive_address
    };

    let directory_tree =
        match Tree::from_datamap_or_address(&client, datamap_chunk, archive_address).await {
            Ok(directory_tree) => directory_tree,
            Err(e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    rest_operation.to_string(),
                    &format!("{rest_operation} failed to get directory Archive - {e}"),
                )
            }
        };

    let (datamap_chunk, data_address, content_type) =
        match directory_tree.lookup_file(&file_path, false) {
            Ok((datamap_chunk, data_address, content_type)) => {
                (datamap_chunk, data_address, content_type)
            }
            Err(_e) => {
                return make_error_response_page(
                    None,
                    &mut HttpResponse::NotFound(),
                    rest_operation.to_string(),
                    "{rest_operation} file not found in directory",
                )
            }
        };

    let content_type = if content_type.is_some() {
        content_type.unwrap().clone()
    } else {
        String::from("text/plain")
    };

    let content = match get_content_using_hex(&client, datamap_chunk, data_address).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return make_error_response_page(
                None,
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!("{rest_operation} failed to get file from network - {e}"),
            );
        }
    };

    HttpResponseBuilder::new(StatusCode::OK)
        .insert_header((header::CONTENT_TYPE, content_type.as_str()))
        .body(content)
}
