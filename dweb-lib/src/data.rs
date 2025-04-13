/*
Copyright (c) 2024-2025 Mark Hughes

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

use autonomi::client::data::DataAddress;
use bytes::Bytes;
use color_eyre::Result;

use autonomi::client::GetError;

use crate::client::DwebClient;

/// TODO: move to dweb::data or similar?
pub async fn autonomi_get_file_public(
    client: &DwebClient,
    file_address: &DataAddress,
) -> Result<Bytes, GetError> {
    println!("DEBUG autonomi_get_file_public()");
    println!("DEBUG calling client.data_get_public()");
    match client.client.data_get_public(file_address).await {
        Ok(content) => {
            println!("DEBUG Ok() return");
            Ok(content)
        }
        Err(e) => {
            println!("DEBUG Err() return");
            Err(e)
        }
    }
}
