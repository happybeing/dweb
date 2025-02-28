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

pub mod fetch;
pub mod name;
pub mod request;

// Default ports for HTTP / HTTPS
pub const DEFAULT_HTTP_PORT: u16 = 8080;
pub const DEFAULT_HTTPS_PORT: u16 = 8443;
pub const LOCALHOST_STR: &str = "127.0.0.1";

// We have two server options, both can be running simultaneously on different ports

// With ports server settings
//
// A random port we expect to be free (see: https://stackoverflow.com/questions/10476987/best-tcp-port-number-range-for-internal-applications)
// This default must be used by *both* 'dweb serve-quick' and 'dweb browse-quick'
// so if it is overridden on the command line, it must be overridden for both commands.
pub const SERVER_PORTS_MAIN_PORT: u16 = 8080;
pub const SERVER_PORTS_MAIN_PORT_STR: &str = "8080";

// With names server settings (deprecated)
//
// Note: unless resurrected, 'with names' features are for testing
// this alternative and should be treated as deprecated. The with
// ports approach is preferred because it simplifies the UX by
// eliminating the need to set-up a local DNS.
pub const SERVER_NAMES_MAIN_PORT: u16 = 8081;
pub const SERVER_NAMES_MAIN_PORT_STR: &str = "8081";

pub const DWEB_SERVICE_WWW: &str = "www-dweb.au";
pub const DWEB_SERVICE_API: &str = "api-dweb.au";
pub const DWEB_SERVICE_APP: &str = "app-dweb.au";
