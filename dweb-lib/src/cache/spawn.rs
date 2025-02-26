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

// TODO implement a lazy static map of handles and basic details of each spawned server
// TODO when the main server app shuts down it can shut these down (if that is needed?)
// TODO web API and CLI for listing active ports and what they are serving
// TODO see TODOs in serve_quick()

pub fn is_main_server_quick_running() -> bool {
    return true; // TODO look-up the main server in the spawned servers struct
}
