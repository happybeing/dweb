// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

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
