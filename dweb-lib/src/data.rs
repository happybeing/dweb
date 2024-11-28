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

use bytes::Bytes;
use color_eyre::Result;
use xor_name::XorName;

use autonomi::client::data::GetError;
use autonomi::client::Client;

/// TODO: move to dweb::data or similar?
pub async fn autonomi_get_file(xor_name: XorName, client: &Client) -> Result<Bytes, GetError> {
    println!("DEBUG autonomi_get_file()");
    println!("DEBUG calling client.data_get()");
    match client.data_get(xor_name).await {
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
