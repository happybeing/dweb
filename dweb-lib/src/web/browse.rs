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

use ant_protocol::storage::PointerAddress as HistoryAddress;

// Cross platform open browser (assumes dweb serve is running)

pub fn handle_open_browser(
    _dweb_name: String,
    _history_address: Option<HistoryAddress>,
    // _directory_address: Option<XorName>, // Only if I support feature("fixed-dweb-hosts")
) {
    // For opening on different platforms:
    //  programmatic?
    //  command line - see https://stackoverflow.com/a/38147878
    println!("TODO - construct URL for dweb and open in a browser on the current platform");
}
