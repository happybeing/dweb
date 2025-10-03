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
    http::{header::ContentType, StatusCode},
    web::Data,
    HttpRequest, HttpResponse,
};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::services::helpers::*;

/// Return address and balance for the wallet being used by dweb
///
/// WARNING: If no wallet was found when starting dweb, a random wallet
/// is created. YOU SHOULD NOT SEND FUNDS to the random wallet as they
/// cannot be recovered once dweb shuts down.
#[utoipa::path(
    responses(
        (status = StatusCode::OK, description = "Success", body = [DwebWallet]),
        ),
    tags = ["Dweb Autonomi"],
)]
#[get("/wallet-balance")]
pub async fn wallet_balance_get(
    request: HttpRequest,
    client: Data<dweb::client::DwebClient>,
) -> HttpResponse {
    println!("DEBUG {}", request.path());

    let rest_operation = "/wallet balance GET errror";
    let rest_handler = "wallet_balance_get()";

    let wallet_address = client.wallet.address().to_string();
    let ant_balance = match client.wallet.balance_of_tokens().await {
        Ok(ant) => format!("{:.28}", f32::from(ant) / 1e18),
        Err(_e) => "error".to_string(),
    };

    let eth_balance = match client.wallet.balance_of_gas_tokens().await {
        Ok(gas) => format!("{:.28}", f32::from(gas) / 1e18),
        Err(_e) => "error".to_string(),
    };

    let dweb_wallet = DwebWallet {
        wallet_address,
        ant_balance,
        eth_balance,
    };

    let json = match serde_json::to_string(&dweb_wallet) {
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

    println!("DEBUG DwebWallet as JSON: {json:?}");

    HttpResponse::Ok()
        .insert_header(ContentType(mime::APPLICATION_JSON))
        .body(json)
}

/// A representation of the Autonomi Wallet for web clients
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct DwebWallet {
    wallet_address: String,
    ant_balance: String,
    eth_balance: String,
}
