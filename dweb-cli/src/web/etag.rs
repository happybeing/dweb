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
use std::hash::{DefaultHasher, Hasher};

use actix_web::{
    http::header,
    http::header::{ETag, EntityTag},
    http::StatusCode,
    HttpRequest, HttpResponse, HttpResponseBuilder,
};

use autonomi::data::private::DataMapChunk;
use autonomi::data::DataAddress;

const ETAG_ADDRESS_LEN: usize = 10; // Length of the abridged data address part of an ETag

/// Provide an ETag that will allow conditional requests for immutable data when
/// accessed via a mutable indirection (e.g. versioned History or Register)
///
/// Where the data requested is immutable, we never need to do eTag matching to know if it has
/// changed but when the version requested is most_recent rather than an explicit version the
/// data requested is not immutable
///
/// IMPORTANT: this assumes different content types are NOT allowed because then the comparison
/// would need to be made in case the response content type is different. At this time there
/// is no way for the REST API to respond with different content types, but if that changes
/// this response method MUST NOT BE USED.
///
pub(crate) fn etag(
    _request: &HttpRequest,
    // The abbreviated address of immutable data (based on its datamap_chunk or data_address)
    etag_address: String,
    content_type: Option<header::ContentType>,
) -> ETag {
    let type_string: String = if let Some(content_type) = content_type.clone() {
        format!("-{}", content_type.to_string())
    } else {
        "".to_string()
    };

    let version_string = "";
    let mutability = "immutable-";

    let etag = format!("{mutability}{etag_address}{version_string}{type_string}");
    println!("DEBUG: immutable data with eTag: \"{etag}\"");
    header::ETag(EntityTag::new_strong(etag))
}

/// Provide an ETag that will allow conditional requests for a versioned resource (e.g. History or Register)
///
/// Where the data requested is immutable, we never need to do eTag matching to know if it has
/// changed but when the version requested is most_recent rather than an explicit version the
/// data requested is not immutable
///
/// IMPORTANT: this assumes different content types are NOT allowed because then the comparison
/// would need to be made in case the response content type is different. At this time there
/// is no way for the REST API to respond with different content types, but if that changes
/// this response method MUST NOT BE USED.
///
/// For is_versioned data (e.g. History or Register)
///     - when not most_recent the version is known and content will be immutable and a
///     strong validator will be returned
///     - when most_recent version is indicated, the current most recent version must have been
///     determined and passed as Some(version> and a weak validator will be inserted into the
///     response. This allows the server to decide whether or not to return 304 Not Modified,
///     or to check the most recent. It could also set up a loop to check the most recent version
///     of the resource and if this has changed, return a response with the latest data and an
///     updated weak validator. (TODO:) This is NOT YET IMPLEMENTED so for now, the server will always
///     re-fetch most_recent versioned data and return the corresponding weak validator.
///
pub(crate) fn versioned_etag(
    _request: &HttpRequest,
    // The abbreviated address of immutable data (based on its datamap_chunk or data_address)
    etag_address: String,
    content_type: Option<header::ContentType>,
    // For versioned data...
    actual_version: u64,
) -> ETag {
    let type_string: String = if let Some(content_type) = content_type.clone() {
        format!("-{}", content_type.to_string())
    } else {
        "".to_string()
    };

    // let version_qualifier = if most_recent { "-latest" } else { "-actual" };
    let version_string: String = format!("-v{actual_version}");

    let etag = format!("immutable{etag_address}{version_string}{type_string}");
    // if most_recent {
    //     println!("DEBUG: most recent version eTag: W/\"{etag}\"");
    //     header::ETag(EntityTag::new_weak(etag))
    // } else {
    println!("DEBUG: immutable data with eTag: \"{etag}\"");
    header::ETag(EntityTag::new_strong(etag))
    // }
}

/// Return an abridged address string for use building an ETag value,
/// based on either a datamap_chunk or data_address
pub(crate) fn address(
    datamap_chunk: Option<DataMapChunk>,
    data_address: Option<DataAddress>,
) -> String {
    let address_string = if let Some(datamap_chunk) = datamap_chunk {
        datamap_chunk.address()
    } else if let Some(data_address) = data_address {
        data_address.to_hex()[..ETAG_ADDRESS_LEN].to_string()
    } else {
        "unknown".to_string()
    };

    address_string
}

/// Return an abridged address string for use building an ETag value,
/// based on either a datamap_chunk or data_address
pub(crate) fn address_from_strings(datamap_chunk: &String, data_address: &String) -> String {
    let address_string = if !datamap_chunk.is_empty() {
        match DataMapChunk::from_hex(datamap_chunk) {
            Ok(datamap_chunk) => datamap_chunk.address(),
            Err(_) => {
                println!("BUG: invalid datamap_chunk when creating ETag: {datamap_chunk}");
                "datamap-error".to_string()
            }
        }
    } else if !data_address.is_empty() {
        data_address.to_string()[..ETAG_ADDRESS_LEN].to_string()
    } else {
        "unknown".to_string()
    };

    address_string
}

/// Handle conditional headers for an immutable request
///
/// Returns None if the operation should proceed, or Some HttpResponseBuilder
/// with either a 304 (Not Modified) or 412 (Precondition Failed) if the
/// operation should be pre-empted.
///
/// TODO extend for PUT and POST (OPTIONS?)
/// TODO remove immutable from name
pub(crate) fn immutable_conditional_response(
    request: &HttpRequest,
    match_etag: Option<&ETag>,
) -> Option<HttpResponse> {
    if immutable_if_none_match(request, match_etag) {
        // Condition met, so go ahead with method
        return None;
    }

    // Condition not met, so pre-emptive response
    use actix_web::http::Method;
    match *request.method() {
        Method::GET | Method::HEAD => {
            Some(HttpResponseBuilder::new(StatusCode::NOT_MODIFIED).finish())
        }
        _ => None,
    }
}

/// Check for and handle a conditional If-None-Match request
///
/// The result can be used to determine the appropriate action and response
/// according to which HTTP method is involved (GET/HEAD or POST/PUT/DELETE etc)
///
/// There is no need to provide an ETag for a known immutable resource, but
/// note that for a versioned website the resource may change depending on
/// which version is being accessed. So in that case an ETag based on
/// the immutable address of the resource should be provided.
///
/// If an ETag is provided it will be compared to the ETag(s) included
/// in the request, otherwise the presence of If-None-Match with an
/// ETag will return false (indicating a 304 (Not Modified) response)
///
/// ref: https://datatracker.ietf.org/doc/html/rfc7232#page-14
///
/// Returns true or false in accordance with rfc7232 where:
/// - true means the operation should go ahead and
/// - false should prevent this, and return either 304 (Not Modified) or
/// 412 (Precondition Failed) response status.
///
///
/// TODO Implement versioned_if_none_match() for History/Register based requests
/// TODO If-None-Match is the most relevant for improving speed of access to
/// TODO immutable data in dweb apps, but other conditions may be useful so:
/// TODO provide if_match()
/// TODO provide if_unmodified_since()
pub(crate) fn immutable_if_none_match(request: &HttpRequest, etag: Option<&ETag>) -> bool {
    if let Some(if_none_match) = request.headers().get(header::IF_NONE_MATCH) {
        match if_none_match.to_str() {
            Ok(if_none_match) => {
                if if_none_match == "*" {
                    // rfc: If the field-value is "*", the condition is false if the origin
                    //      server has a current representation for the target resource.
                    // The purpose of this is to prevent 'competing' updates from different
                    // clients interfering and causing loss of an update. This doesn't
                    // apply to immutable data updates so we always return 'true' to allow
                    // the update to go ahead and if the data exists, the update will fail
                    // rather than be 'lost'.
                    return true;
                }
                if let Some(etag) = etag {
                    let etag = etag.to_string();
                    for tag in if_none_match.split(',') {
                        println!("DEBUG ETAG COMPARING: {} and {}", tag, etag);
                        if tag == etag {
                            return false;
                        }
                    }
                    return true;
                }

                // When no ETag is provided we can assume the data cannot change
                return false;
            }
            Err(_e) => {
                // Condition value invalid so ignore it
            }
        }
    }
    return true; // Default is to go ahead when not prevented by this header
}

pub(crate) fn invalid_header_response() -> HttpResponse {
    HttpResponse::BadRequest().finish()
}

pub(crate) fn etag_for_address(data_address: &DataAddress) -> ETag {
    ETag(EntityTag::new_strong(data_address.to_hex().to_owned()))
}

pub(crate) fn etag_for_datamap_chunk(datamap_chunk: &DataMapChunk) -> ETag {
    let mut hasher = DefaultHasher::new();
    hasher.write(datamap_chunk.to_hex().as_bytes());
    let hash = format!("{:64x}", hasher.finish());
    ETag(EntityTag::new_strong(hash))
}
