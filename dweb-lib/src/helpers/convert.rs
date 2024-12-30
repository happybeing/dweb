// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use std::f64::MAX_10_EXP;

use ant_registers::{Entry, RegisterAddress};
use color_eyre::eyre::{eyre, Result};
use xor_name::XorName;

// The following functions copied from sn_cli with minor changes (eg to message text)

/// Parse a hex register address with optional URL scheme
/// TODO modify for dweb use: Parse a hex register address with optional URL scheme
pub fn str_to_register_address(str: &str) -> Result<RegisterAddress> {
    // let str = if str.starts_with(AWE_PROTOCOL_REGISTER) {
    //     &str[AWE_PROTOCOL_REGISTER.len()..]
    // } else {
    //     &str
    // };

    match RegisterAddress::from_hex(str) {
        Ok(register_address) => Ok(register_address),
        Err(e) => Err(eyre!("Invalid register address string '{str}':\n{e:?}")),
    }
}

/// TODO modify for dweb use: Parse a hex xor address with optional URL scheme
pub fn str_to_xor_name(str: &str) -> Result<XorName> {
    // let mut str = if str.starts_with(AWE_PROTOCOL_METADATA) {
    //     &str[AWE_PROTOCOL_METADATA.len()..]
    // } else if str.starts_with(AWE_PROTOCOL_FILE) {
    //     &str[AWE_PROTOCOL_FILE.len()..]
    // } else {
    //     &str
    // };
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match hex::decode(str) {
        Ok(bytes) => match bytes.try_into() {
            Ok(xor_name_bytes) => Ok(XorName(xor_name_bytes)),
            Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
        },
        Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
    }
}

////// awe protocol versions of the above for use by dweb CLI

pub const AWE_PROTOCOL_REGISTER: &str = "awv://";
#[allow(dead_code)]
pub const AWE_PROTOCOL_METADATA: &str = "awm://";
#[allow(dead_code)]
pub const AWE_PROTOCOL_FILE: &str = "awf://";

// Default ports for HTTP / HTTPS
pub const DEFAULT_HTTP_PORT_STR: &str = "8080";
pub const DEFAULT_HTTPS_PORT_STR: &str = "8443";
pub const LOCALHOST: &str = "127.0.0.1";

// Assignable port range (https://en.wikipedia.org/wiki/Registered_port)
pub const MIN_SERVER_PORT: u16 = 1024;
pub const MAX_SERVER_PORT: u16 = 49451;

/// Parse a port number for a server to listen on
pub fn parse_port_number(str: &str) -> Result<u16> {
    let port = str.parse::<u16>()?;

    if port >= MIN_SERVER_PORT && port <= MAX_SERVER_PORT {
        Ok(port)
    } else {
        Err(eyre!(
            "Invalid port number. Valid numbers are {MIN_SERVER_PORT}-{MAX_SERVER_PORT}"
        ))
    }
}

/// Parse a hostname for a server to listen on
pub fn parse_host(hostname: &str) -> Result<String> {
    let host = hostname.parse::<String>()?;

    match url::Url::parse(&host) {
        Ok(_url) => Ok(String::from(hostname)),
        Err(e) => Err(eyre!(e)),
    }
}

/// Parse a hex register address with optional URL scheme
pub fn awe_str_to_register_address(str: &str) -> Result<RegisterAddress> {
    let str = if str.starts_with(AWE_PROTOCOL_REGISTER) {
        &str[AWE_PROTOCOL_REGISTER.len()..]
    } else {
        &str
    };

    match RegisterAddress::from_hex(str) {
        Ok(register_address) => Ok(register_address),
        Err(e) => Err(eyre!("Invalid register address string '{str}':\n{e:?}")),
    }
}

pub fn awe_str_to_xor_name(str: &str) -> Result<XorName> {
    let mut str = if str.starts_with(AWE_PROTOCOL_METADATA) {
        &str[AWE_PROTOCOL_METADATA.len()..]
    } else if str.starts_with(AWE_PROTOCOL_FILE) {
        &str[AWE_PROTOCOL_FILE.len()..]
    } else {
        &str
    };
    let str = if str.ends_with('/') {
        &str[0..str.len() - 1]
    } else {
        str
    };

    match hex::decode(str) {
        Ok(bytes) => match bytes.try_into() {
            Ok(xor_name_bytes) => Ok(XorName(xor_name_bytes)),
            Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
        },
        Err(e) => Err(eyre!("XorName not valid due to {e:?}")),
    }
}

// From FoldersApi
// Helper to convert a Register/Folder entry into a XorName
pub fn xorname_from_entry(entry: &Entry) -> XorName {
    let mut xorname = [0; xor_name::XOR_NAME_LEN];
    xorname.copy_from_slice(entry);
    XorName(xorname)
}
