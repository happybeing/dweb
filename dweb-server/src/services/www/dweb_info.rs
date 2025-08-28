/*
 Copyright (c) 2025 Mark Hughes

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

use actix_web::{get, web::Data, HttpRequest, HttpResponse, Responder};
use qstring::QString;

use dweb::cache::directory_with_port::*;
use dweb::files::directory::Tree;
use dweb::history::History;

use super::make_error_response_page;

/// Show information about the current directory or website
///
/// /dweb-info[?use-graph=true|false]
///
/// If use-graph is 'true' it traverses the graph to the end to find the 'head' entry
/// rather than use the pointer. This causes a delay but can be useful as the pointer may
/// not be up-to-date.
///
/// url: <code>http://127.0.0.1:<PORT-NUMBER>/dweb-info</code>
#[utoipa::path(
    responses(
        (status = StatusCode::OK,
            description = "HTML summary of the directory or website History", body = str)
        ),
    tags = ["Manual"],
    params(
        ("use-graph" = Option<bool>, description = "when 'true' ignores the Pointer and follows the graph to find the most recent entry in the content History"),
    )
)]
#[get("/dweb-info")]
pub async fn dweb_info(
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
    our_directory_version: Data<Option<DirectoryVersionWithPort>>,
    _is_local_network: Data<bool>,
    _is_main_server: Data<bool>,
) -> impl Responder {
    println!("DEBUG dweb_info()...");

    let directory_version = if our_directory_version.is_some() {
        our_directory_version.as_ref().clone().unwrap()
    } else {
        return make_error_response_page(
            None,
            &mut HttpResponse::InternalServerError(),
            "/dweb_version error".to_string(),
            &format!("Unable to access our_directory_version - probably a bug"),
        );
    };

    println!("DEBUG {directory_version}");

    let qs = QString::from(request.query_string());
    let use_graph: bool = match qs.get("use-graph").unwrap_or("false") {
        "true" => true,
        "1" => true,
        _ => false,
    };

    let (pointer_max_version, graph_max_version) =
        if let Some(history_address) = directory_version.history_address {
            let pointer_max_version;
            let mut graph_max_version = "not checked".to_string();
            let client = client.as_ref().clone();
            match History::<Tree>::from_history_address(
                client.clone(),
                history_address,
                false,
                directory_version.version.unwrap_or(1),
            )
            .await
            {
                Ok(history) => match history.num_versions() {
                    Ok(num_versions) => pointer_max_version = format!("{}", num_versions),
                    Err(e) => pointer_max_version = format!("unknown (error: {e}"),
                },
                Err(e) => {
                    return make_error_response_page(
                        None,
                        &mut HttpResponse::InternalServerError(),
                        "/dweb_version error".to_string(),
                        &format!(
                            "failed to get History from address '{}': {e} - probably a bug",
                            history_address.to_hex()
                        ),
                    );
                }
            };

            if use_graph {
                match History::<Tree>::from_history_address(
                    client.clone(),
                    history_address,
                    use_graph,
                    directory_version.version.unwrap_or(1),
                )
                .await
                {
                    Ok(history) => match history.num_versions() {
                        Ok(num_versions) => graph_max_version = format!("{}", num_versions),
                        Err(e) => graph_max_version = format!("unknown (error: {e}"),
                    },
                    Err(e) => {
                        return make_error_response_page(
                            None,
                            &mut HttpResponse::InternalServerError(),
                            "/dweb_version error".to_string(),
                            &format!(
                                "failed to get History from address '{}': {e} - probably a bug",
                                history_address.to_hex()
                            ),
                        );
                    }
                }
            };
            (pointer_max_version, graph_max_version)
        } else {
            ("unkown".to_string(), "unknown".to_string())
        };

    make_dweb_info_response(&directory_version, &pointer_max_version, &graph_max_version)
}

fn make_dweb_info_response(
    directory_version: &DirectoryVersionWithPort,
    pointer_max_version: &str,
    graph_max_version: &str,
) -> HttpResponse {
    let body = if directory_version.history_address.is_some() {
        let heading = "/dweb-info for History";
        let version_str = if directory_version.version.is_some() {
            &format!("{}", directory_version.version.unwrap())
        } else {
            "most recent"
        };

        format!(
            "
            <!DOCTYPE html><title>{heading}</title><body>
            <h3>{heading}</h3>
            HistoryAddress: {}<br/>
            ArchiveAddress: {}<br/>
            Current version: {}<br/>
            <br/>
            Max version from pointer: {}<br/>
            Max version from graph: {}<br/>
            <br/>
            <p>
            To get the max version from graph, in the address bar replace /dweb-info with /dweb-info?use-graph=1
            </p>

            <br/><br/><a href='javascript:history.back()'>Go back</a>
            </body>",
            directory_version.history_address.unwrap().to_hex(),
            directory_version.archive_address.to_hex(),
            version_str,
            pointer_max_version,
            graph_max_version,
        )
    } else {
        let heading = "/dweb-info for Directory";
        format!(
            "
            <!DOCTYPE html><title>{heading}</title><body>
            <h3>{heading}</h3>
            ArchiveAddress: {}<br/>
            <br/><br/><a href='javascript:history.back()'>Go back</a>
            </body>",
            directory_version.archive_address.to_hex()
        )
    };

    HttpResponse::Ok().body(body)
}
