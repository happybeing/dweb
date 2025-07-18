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

use std::io::Read;

use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{
    http::{header::ContentType, StatusCode},
    put, web,
    web::Data,
    HttpRequest, HttpResponse,
};
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};
use utoipa::{schema, ToSchema};

use dweb::helpers::retry::retry_until_ok;
use dweb::{client::DwebClient, token::format_tokens_as_attos};

use super::{DwebType, MutateResult};
use crate::services::helpers::*;

#[derive(Deserialize, ToSchema)]
struct QueryParams {
    tries: Option<u32>,
}
// NOTES:
//  To derive ToSchema can try:
//      Building the schema or faking the struct: https://github.com/juhaku/utoipa/discussions/742
//      Using #[schema(...)] (see https://docs.rs/utoipa/latest/utoipa/derive.ToSchema.html#mixed-enum-unit-field-variant-optional-configuration-options-for-serdeschema)
#[derive(Debug, MultipartForm, ToSchema)]
struct UploadForm {
    #[multipart(limit = "100MB")] // TODO remove limit when streaming supported in Autonomi APIs
    #[schema(value_type = String, format = Binary)]
    file: TempFile,
    // #[schema(value_type = String)]
    // name: Text<String>,
}

/// Multipart form upload of a single file (as public or private)
///
/// Note: you can use this API to PUT data from memory instead of
/// a file by using JavaScript. Either with a FormData object, or by
/// setting properties on an input element.
///
/// Example form:
/// ```
/// <form target="/form-upload-file/true" method="put" enctype="multipart/form-data">
///     <input type="file" name="file"/>
///     <button type="submit">Submit</button>
/// </form>
/// ```
//
// Note:
// I have chosen to use PUT rather than POST here to associate this with the terminology of the Autonomi APIs
// such as data_put_public().
//
// While the effect is no different, POST implies creating a resource and PUT is for updating, but there is no
// compelling reason for choosing one over the other here. Also note that the rules for this are not as certain
// as some will say. For example, see: Roy T. Fielding here: https://roy.gbiv.com/untangled/2009/it-is-okay-to-use-put
//
#[utoipa::path(
// See: https://github.com/juhaku/utoipa/discussions/742
//    request_body(content = WhatEverStruct, description = "Multipart file", content_type = "multipart/form-data"),
    put,
    params(
        ("make_public" = bool, description = "true to upload data as public"),
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each file upload, 0 means unlimited. This overrides the API control setting in the server.")),
    request_body(content = UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = StatusCode::CREATED, description = "A MutateResult featuring either status 201 with cost and data address on the network, or in case of error an error status code and message about the error.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;INTERNAL_SERVER_ERROR: Error reading file or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;BAD_GATEWAY: Autonomi network error", body = MutateResult,
            example = json!("{\"file_name\": \"somefile.txt\", \"status\": \"201\", \"cost_in_ant\": \"12\", \"data_address\": \"a9cd8dd0c9f2b9dc71ad548d1f37fcba6597d5eb1be0b8c63793802cc6c7de27\", \"data_map\": \"\", \"message\": \"\" }")),
    ),
    tags = ["Dweb"],
)]
#[put("/form-upload-file/{make_public}")]
pub async fn data_put(
    MultipartForm(mut form): MultipartForm<UploadForm>,
    request: HttpRequest,
    path_params: web::Path<bool>,
    query_params: web::Query<QueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    let make_public = path_params.into_inner();
    let tries = query_params.tries.unwrap_or(client.api_control.api_tries);

    println!("DEBUG {}", request.path());
    let mutate_result = if make_public {
        put_file_public(&client, &mut form.file, tries).await
    } else {
        put_file_private(&client, &mut form.file, tries).await
    };

    mutate_result.make_response("/form-upload-file PUT error", "data_put()")
}

#[derive(Debug, MultipartForm, ToSchema)]
struct UploadFormList {
    // #[multipart(rename = "file")]
    #[multipart(limit = "100MB")] // TODO remove limit when streaming supported in Autonomi APIs
    #[schema(value_type = Vec<String>, format = Binary)]
    files: Vec<TempFile>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MutateResultList {
    mutate_results: Vec<MutateResult>,
}

/// Multipart form upload of one or more files (as public or private)
///
/// Note: for large datasets, you can use this API to PUT data from memory
/// instead of a file by using JavaScript. Either with a FormData object,
/// or by setting properties on an input element.
///
/// Example form:
/// ```
/// <form target="/form-upload-file-list/true" method="put" enctype="multipart/form-data">
///     <input type="file" multiple name="file"/>
///     <button type="submit">Submit</button>
/// </form>
/// ```
#[utoipa::path(
// See: https://github.com/juhaku/utoipa/discussions/742
//    request_body(content = WhatEverStruct, description = "Multipart file", content_type = "multipart/form-data"),
    put,
    params(
        ("make_public" = bool, description = "true to upload data as public"),
        ("tries" = Option<u32>, Query, description = "number of times to try calling the Autonomi upload API for each file upload, 0 means unlimited. This overrides the API control setting in the server.")),
    request_body(content = UploadFormList, content_type = "multipart/form-data"),
    responses(
        (status = StatusCode::CREATED, description = "Returned if any successful storage occurs. A MutateResultList is returned featuring a MutateResult for each upload either status 201 with cost and data address on the network, or in case of error an error status code and message about the error. Inspect the individual MutateResult.status_code values to see which have been successful.<br/>\
        <b>Error StatusCodes</b><br/>\
        &nbsp;&nbsp;&nbsp;INTERNAL_SERVER_ERROR: Error reading file or storing in memory<br/>\
        &nbsp;&nbsp;&nbsp;BAD_GATEWAY: Autonomi network error", body = [MutateResultList],
            example = json!("{\"mutate_results\": [{\"file_name\": \"somefile.txt\", \"status\": \"201\", \"cost_in_ant\": \"12\", \"data_address\": \"a9cd8dd0c9f2b9dc71ad548d1f37fcba6597d5eb1be0b8c63793802cc6c7de27\", \"data_map\": \"\", \"message\": \"\" }]}")),
    ),
    tags = ["Dweb"],
)]
#[put("/form-upload-file-list/{make_public}")]
pub async fn data_put_list(
    MultipartForm(form): MultipartForm<UploadFormList>,
    request: HttpRequest,
    path_params: web::Path<bool>,
    query_params: web::Query<QueryParams>,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());
    let rest_operation = "/form-upload-file-list PUT";
    let rest_handler = "data_put_list()";

    let make_public = path_params.into_inner();
    let tries = query_params.tries.unwrap_or(client.api_control.api_tries);

    let mut put_list = MutateResultList {
        mutate_results: Vec::<MutateResult>::new(),
    };
    for mut file in form.files {
        println!(
            "DEBUG data_put_list() file: {:?}, size: {}",
            file.file_name, file.size
        );
        let mutate_result = if make_public {
            put_file_public(&client, &mut file, tries).await
        } else {
            put_file_private(&client, &mut file, tries).await
        };
        put_list.mutate_results.push(mutate_result);
    }

    let json = match serde_json::to_string(&put_list) {
        Ok(json) => json,
        Err(e) => {
            return make_error_response_page(
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                &mut HttpResponse::NotFound(),
                rest_operation.to_string(),
                &format!("{rest_handler} failed to encode JSON result - {e}"),
            )
        }
    };

    println!("DEBUG response MutateResultList as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

async fn put_file_public(client: &DwebClient, file: &mut TempFile, tries: u32) -> MutateResult {
    let dweb_type = DwebType::PublicFile;

    // TODO update if Autonomi supports streamed uploads
    let mut content = Vec::<u8>::new();
    let file_name = file.file_name.clone().unwrap_or("unknown".to_string());
    let content_len = match file.file.read_to_end(&mut content) {
        Ok(content_len) => content_len,
        Err(e) => {
            let status_message =
                format!("put_file_public() failed to read file '{file_name}' into buffer - {e}");
            println!("DEBUG {status_message}");
            return MutateResult {
                dweb_type,
                status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                status_message,
                file_name,
                ..Default::default()
            };
        }
    };

    let data = content.into();
    let payment_option = client.payment_option().clone();
    let result = retry_until_ok(
        tries,
        &"data_put_public()",
        (data, payment_option),
        async move |(data, payment_option)| match client
            .client
            .data_put_public(data, payment_option.clone())
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(eyre!(e)),
        },
    )
    .await;

    match result {
        Ok(result) => {
            println!("DEBUG put_file_public() stored '{file_name}' {content_len} bytes on the network at address");
            MutateResult {
                dweb_type,
                status_code: StatusCode::CREATED.as_u16(),
                status_message: "success".to_string(),
                cost_in_ant: format_tokens_as_attos(result.0.as_atto()),
                file_name,
                network_address: result.1.to_hex(),
                ..Default::default()
            }
        }
        Err(e) => {
            let status_message =
                format!("put_file_public() failed store file '{file_name}' on the network - {e}");
            println!("DEBUG {status_message}");
            MutateResult {
                dweb_type,
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                status_message,
                file_name,
                ..Default::default()
            }
        }
    }
}

async fn put_file_private(client: &DwebClient, file: &mut TempFile, tries: u32) -> MutateResult {
    let dweb_type = DwebType::PrivateFile;

    // TODO update if Autonomi supports streamed data_put() (or file_put_public())
    let mut content = Vec::<u8>::new();
    let file_name = file.file_name.clone().unwrap_or("unknown".to_string());
    let content_len = match file.file.read_to_end(&mut content) {
        Ok(content_len) => content_len,
        Err(e) => {
            let status_message =
                format!("put_file_private() failed to read file '{file_name}' into buffer - {e}");
            println!("DEBUG {status_message}");
            return MutateResult {
                dweb_type,
                status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                status_message,
                file_name,
                ..Default::default()
            };
        }
    };

    let data = content.into();
    let payment_option = client.payment_option().clone();
    let result = retry_until_ok(
        tries,
        &"data_put()",
        (data, payment_option),
        async move |(data, payment_option)| match client
            .client
            .data_put(data, payment_option.clone())
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(eyre!(e)),
        },
    )
    .await;

    match result {
        Ok(result) => {
            println!("DEBUG put_file_private() stored '{file_name}' {content_len} bytes on the network at address");
            MutateResult {
                dweb_type,
                status_code: StatusCode::CREATED.as_u16(),
                status_message: "success".to_string(),
                cost_in_ant: format_tokens_as_attos(result.0.as_atto()),
                file_name,
                data_map: result.1.to_hex(),
                ..Default::default()
            }
        }
        Err(e) => {
            let status_message =
                format!("put_file_private() failed store file '{file_name}' on the network - {e}");
            println!("DEBUG {status_message}");
            MutateResult {
                dweb_type,
                status_code: StatusCode::BAD_GATEWAY.as_u16(),
                status_message,
                file_name,
                ..Default::default()
            }
        }
    }
}
