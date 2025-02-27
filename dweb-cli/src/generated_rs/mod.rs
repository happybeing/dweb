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

pub(crate) mod builtins_local;
pub(crate) mod builtins_public;

use crate::services::register_name;

// Register builtin history addresses so they can be used immediately in browser (and CLI if supported in cli_options.rs)
pub fn register_builtin_names(is_local: bool) {
    use crate::generated_rs::{builtins_local, builtins_public};

    if is_local {
        register_name("awesome", builtins_local::AWESOME_SITE_HISTORY_LOCAL);
    } else {
        register_name("awesome", builtins_public::AWESOME_SITE_HISTORY_PUBLIC);
    }
}
