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

use std::future::Future;

use color_eyre::eyre::{eyre, Report, Result};

/// Retry a closure until Ok(<RETURN>) or tries is reached
/// If tries is 0, only returns on success
pub async fn retry_until_ok<F, Fut, Params: Clone, R>(
    tries: u32,
    label: &str,
    params: Params,
    f: F,
) -> Result<R, Report>
where
    F: Fn(Params) -> Fut,
    Fut: Future<Output = Result<R, Report>>,
{
    let tries_string = if tries == 0 {
        "unlimited".to_string()
    } else {
        format!("{tries}")
    };

    let mut try_number = 1;
    let mut last_error = eyre!("retry_until_ok() - hit a bug!");
    println!(">>TRYING {label} {tries_string} times: ");
    while tries == 0 || try_number <= tries {
        match f(params.clone()).await {
            Ok(result) => {
                println!(">>SUCCESS!");
                return Ok(result);
            }
            Err(e) => last_error = eyre!(format!(">>{tries_string} complete with error - {e}")),
        }
        try_number = try_number + 1;
    }
    Err(last_error)
}
