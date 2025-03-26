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

use utoipa::OpenApi;

pub(crate) const JSON_PATH: &str = "/api/openapi.json";
pub(crate) const SWAGGER_UI: &str = "/swagger-ui/#/";

#[derive(Debug, OpenApi)]
#[openapi(info(
    title = "dweb",
    description = "
### A RESTful API for the Autonomi peer-to-peer network
<p>
This RESTful API is part of a package of features for both users and develpers using Autonomi.
</p>
<p>
<b>dweb</b> is:
</p>
<li>an app for publishing versioned, decentralised websites and web apps on Autonomi</li>
<li>a local server for viewing Autonomi websites, apps and data in a regular browser</li>
<li>a RESTful API for accessing the Autonomi APIs, and dweb APIs built on top of those</li>
<li>a Rust crate that simplifies building Autonomi apps, and adds features such as versioned data types</li>
<li>a command line app with features for users and developers</li>
\n
More on github: [https://github.com/happybeing/dweb/dweb-cli](https://github.com/happybeing/dweb/tree/main/dweb-cli#dweb-command-line-app)"
),
tags(
    [name = "manual", description = "simplified APIs for manual input into the address bar"],
    [name = dweb::api::DWEB_API_ROUTE, description = "dweb APIs"],
    // [name = dweb::api::ANT_API_ROUTE, description = "dweb wrappers for Autonomi APIs"],
    // [name = "default", description = "untagged APIs"],
),
)]
pub(crate) struct DwebApiDoc;
