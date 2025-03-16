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

pub mod directory;
pub mod name;

use actix_web::{
    dev::HttpServiceFactory, get, web, web::Data, HttpRequest, HttpResponse, Responder,
};

use dweb::api::DWEB_API_ROUTE;

pub fn init_service() -> impl HttpServiceFactory {
    // TODO modify this and the get to accept /{api}/{version}/{operation} etc (see www::init_service())
    actix_web::web::scope(DWEB_API_ROUTE)
        .service(name::api_dwebname_register)
        .service(name::api_dwebname_list)
        .service(actix_web::web::scope("/directory-load").service(directory::api_directory_load))
}

#[get("/test/unsupported/route")]
pub async fn api_test_no_route(
    _request: HttpRequest,
    _params: web::Path<(String, String)>,
    _client_data: Data<dweb::client::AutonomiClient>,
) -> impl Responder {
    HttpResponse::Ok().body("dweb serve: ROUTE NOT IMPLEMENTED")
}
