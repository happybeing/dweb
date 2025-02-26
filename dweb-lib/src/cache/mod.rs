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

//! A set of caches which serve to speed up access to data, and in
//! the case of websites, enable access to different versions of a
//! website to work in a standard browser. Without a way to map the
//! 'host' part of a URL to a VERSION and HISTORY-ADDRESS, it would
//! not be possible for a dweb server to know which version of a
//! website an HttpRequest was related to.
//!
//! TODO: consider persisting the caches (do any feature serde?)

// This module includes these cache implementations:

pub mod directory;
pub mod directory_with_name;
pub mod directory_with_port;
pub mod file;
pub mod spawn;
