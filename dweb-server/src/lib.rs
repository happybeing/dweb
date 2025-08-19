/*
*   Copyright (c) 2025 Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU Affero General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU Affero General Public License for more details.

*   You should have received a copy of the GNU Affero General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

mod helpers;
mod services;
mod web;

use dweb::client::DwebClient;

#[derive(Debug)]
pub enum DwebServiceError {
    NOT_STARTED,
}
///
///
///
pub struct DwebService {
    dweb_client: DwebClient,
    is_started: bool,
}

impl DwebService {
    pub fn new(dweb_client: DwebClient) -> DwebService {
        DwebService {
            dweb_client,
            is_started: false,
        }
    }

    pub async fn start(&mut self) -> Result<(), DwebServiceError> {
        match crate::services::init_dweb_server(&self.dweb_client).await {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("DEBUG DwebService failed to start: {e}");
                Err(DwebServiceError::NOT_STARTED)
            }
        }
    }

    /// Stop the service
    pub fn stop(&mut self) -> Result<(), DwebServiceError> {
        if self.started() {
            // TODO call REST API telling the service to exit()
            self.is_started = false;
            return Ok(());
        }

        Err(DwebServiceError::NOT_STARTED)
    }

    /// true if the service started successfully (and has not been stopped)
    pub fn started(&self) -> bool {
        return self.is_started;
    }
}
