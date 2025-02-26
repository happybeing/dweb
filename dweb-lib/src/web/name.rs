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

//! # DwebHost
//!
//! The 'host' part of a dweb web URL uses the domain 'www-dweb.au' plus either
//! one or two subdomains:
//!
//!    `[v<VERSION>.]<DWEB-NAME>.www-dweb.au`
//!
//! The first part is an optional followed by a short name (which correponds to a History
//! stored on Autonomi).
//!
//!    VERSION 1 is the first version, 2 the second etc, and if omitted implies 'most recent'.
//!
//!    A DWEB-NAME corresponds to a particular website history. It begins with a memorable part,
//!    a mnemonic for the website, followed by a hyphen and ends with the first few characters from
//!    the xor encoded HistoryAddress. The memorable part is a lowercase alphabetic string which
//!    may be taken from website metadata or specified by the user. The characters after the hyphen
//!    serve to disambiguiate websites which could have the same memorable part.
//!
//! Example web names:
//!    'awesome-f8b3.www-dweb.au'          - the most recent version a website
//!    'v23.awesome-f8b3.www-dweb.au'      - the 23rd version the same website
//!    'v23.awesome-f2e4.www-dweb.au'      - the 23rd version of a different website
//!
//! DwebHosts allow the correct website version to be retrieved from a History<DirectoryTree>
//! on Autonomi and the corresponding content to be returned to a standard web browser. They act
//! as keys for a cache maintained by the local dweb server, but must first be created using
//! the appropriate dweb APIs.
//!
//! Once created, resolving a DwebHost requires a local DNS to redirect the dweb domain www-dweb.au
//! to a local dweb server (e.g. dweb-cli) which decodes the name and accesses the relevant website
//! version from a cache held in the server.
//!
//! DwebHosts could be persisted in various ways, such as in a separate website on Autonomi or
//! the private Vault of a user, which then provides a set of 'favourites' or web bookmarks personal
//! to a user.
//!
//! Without persistence, different DWEB-NAMES can be used with the same HistoryAddress at different
//! times, there is always a one-to-one correspondence between the two, so neither can be coupled
//! to more than one of the other at one time.
//!
//! TODO: implement persistent DWEB-NAMES per user and use to provide a page of sites with brief
//! information to aid identification.
//!

use color_eyre::eyre::{eyre, Result};

use crate::cache::directory_with_name::HISTORY_NAMES;
use crate::trove::HistoryAddress;

// Domain name and subdomain constraints based on IETF RFC1035 with links to relevant sections:
pub const MAX_SUBDOMAIN_LEN: usize = 63; //  S2.3.4 Size limits (https://datatracker.ietf.org/doc/html/rfc1035#section-2.3.4)
                                         // A subdomain must start with a letter (a-z) and is followed by one or more letters or numbers
                                         // which may be separated by a hyphen. S2.3.1. Preferred name syntax (https://datatracker.ietf.org/doc/html/rfc1035#section-2.3.1)

pub const DISAMBIGUATION_LEN: usize = 4; // Number of hexadecimal disambiguation characters to include in a DWEB-NAME
pub const MEMORABLE_PART_LEN: usize = MAX_SUBDOMAIN_LEN - DISAMBIGUATION_LEN - 1; // Allow 1 for hyphen

pub const DOMAIN_PART: &str = "www-dweb";
pub const TLD_PART: &str = "au";

const VERSION_CHAR: u8 = b'v';
const FIXED_WEBNAME_SEPARATOR: &str = "-f";

/// DwebHost corresponds to the HOST part of a dweb URL and encapsulates the component
/// subdomain and domain parts which are used to lookup the content for a version of a
/// website.
///
pub struct DwebHost {
    /// `[v<VERSION>.]<DWEB-NAME>.www-dweb.au`
    pub dweb_host_string: String,
    pub dweb_name: String,
    /// None implies most recent version (highest number)
    pub version: Option<u32>,

    #[cfg(feature = "fixed-dweb-hosts")]
    // Development build feature for non-versioned DirectoryTree references
    pub is_fixed_dweb_host: bool,
}

/// Make a valid DWEB-NAME for a dweb URL
///
/// See validate_dweb_name() for more.
pub fn make_dweb_name(memorable_part: &String, history_address: HistoryAddress) -> Result<String> {
    if memorable_part.len() == 0 {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME must include at least one alphabetic character"
        ));
    }

    if !memorable_part.as_bytes()[0].is_ascii_alphabetic() {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME must begin with an alphabetic character"
        ));
    }

    if !memorable_part.len() > MEMORABLE_PART_LEN {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME cannot exceed {MEMORABLE_PART_LEN} characters"
        ));
    }

    if memorable_part.contains("--") {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME cannot contain consecutive hyphens"
        ));
    }

    if !memorable_part[1..]
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME can only contain alphanumeric characters and hyphens"
        ));
    }

    // Prevent clash with 'fixed version' web names
    if !memorable_part.ends_with(FIXED_WEBNAME_SEPARATOR) {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME cannot end with '{FIXED_WEBNAME_SEPARATOR}'"
        ));
    }

    let history_part = format!("{}", history_address.to_hex());
    Ok(
        memorable_part[..MEMORABLE_PART_LEN].to_string()
            + "-"
            + &history_part[..DISAMBIGUATION_LEN],
    )
}

/// Create a version part ("v[VERSION]") for a www-dweb URL
pub fn make_version_part(version: u32) -> String {
    if version > 0 {
        format!("v{version}")
    } else {
        String::from("")
    }
}

#[cfg(feature = "fixed-dweb-hosts")]
use xor_name::XorName as ArchiveAddress;

pub fn make_fixed_dweb_name(
    memorable_part: &String,
    archive_address: ArchiveAddress,
) -> Result<String> {
    if memorable_part.len() == 0 {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME must include at least one alphabetic character"
        ));
    }

    if !memorable_part.as_bytes()[0].is_ascii_alphabetic() {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME must begin with an alphabetic character"
        ));
    }

    const FIXED_MEMORABLE_PART_LEN: usize = MEMORABLE_PART_LEN - FIXED_WEBNAME_SEPARATOR.len();
    if !memorable_part.len() > FIXED_MEMORABLE_PART_LEN {
        return Err(eyre!(
            "'fixed' version website The memorable part of a DWEB-NAME cannot exceed {FIXED_MEMORABLE_PART_LEN} characters"
        ));
    }

    if memorable_part.contains("--") {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME cannot contain consecutive hyphens"
        ));
    }

    if !memorable_part[1..]
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME can only contain alphanumeric characters and hyphens"
        ));
    }

    // Prevent clash with 'fixed version' web names
    if !memorable_part.ends_with(FIXED_WEBNAME_SEPARATOR) {
        return Err(eyre!(
            "The memorable part of a DWEB-NAME cannot end with '{FIXED_WEBNAME_SEPARATOR}'"
        ));
    }

    let directory_part = format!("{archive_address:x}");
    let web_name = memorable_part[..MEMORABLE_PART_LEN].to_string()
        + FIXED_WEBNAME_SEPARATOR
        + "-"
        + &directory_part[..];

    Ok(web_name.to_ascii_lowercase())
}

/// Decode a dweb host string
/// Returns a DwebHost which includes the validated web name string, DWEB-NAME and VERSION (if present)
///
///  For example, 'v2.awesome-f834.www-dweb.au' would return
///     Ok(DwebHost{
///         dweb_host: &"v2.awesome-f834.www-dweb.au",
///         dweb_name: &"awesome-f834",
///         version: Some(2)
///     })
///
/// # Examples
///
/// ```
/// use crate::dweb::web::name;
/// assert!(name::decode_dweb_host("v2.awesome-f834.www-dweb.au").is_ok());
/// assert!(name::decode_dweb_host("awesome-f834.www-dweb.au").is_ok());
/// assert!(name::decode_dweb_host("awesome99-f834.www-dweb.au").is_ok());
/// assert!(name::decode_dweb_host("awe-some-f834.www-dweb.au").is_ok());
/// assert!(name::decode_dweb_host("awe-99some-f834.www-dweb.au").is_ok());
///
/// assert!(name::decode_dweb_host("9awesome-f834.www-dweb.au").is_err());
/// assert!(name::decode_dweb_host("awe=some-f834.www-dweb.au").is_err());
/// assert!(name::decode_dweb_host("awe--some-f834.www-dweb.au").is_err());
/// assert!(name::decode_dweb_host(&String::from("v.awesome-f834.www-dweb.au").as_str()).is_err());
///
/// ```
//
// Note: for --features=fixed-dweb-names, this will also decode fixed web names which are
// differentiated by a DWEB-NAME containing the String::from(FIXED_WEBNAME_SEPARATOR) + "-";
pub fn decode_dweb_host(dweb_host: &str) -> Result<DwebHost> {
    println!("DEBUG decode_dweb_host({dweb_host})...");
    if dweb_host.len() == 0 {
        return Err(eyre!("Dweb host cannot be zero length"));
    }

    let fixed_dweb_host_tag = String::from(FIXED_WEBNAME_SEPARATOR) + "-";

    let mut segments = dweb_host.split('.');
    let total_segments = segments.clone().count();
    if total_segments > 4 || total_segments < 3 {
        return Err(eyre!(
            "Dweb host must contain three or four segments, each separated by '.'"
        ));
    }

    let mut found_version_segment = false;
    // If four segments are present, process the first as 'v<VERSION>'
    let version = if segments.clone().count() == 4 && dweb_host.as_bytes()[0] == VERSION_CHAR {
        match segments.next() {
            Some(str) => {
                if !str.starts_with('v') {
                    return Err(eyre!(
                        "Dweb host contains four segments (separated by '.') so first must start with 'v'"
                    ));
                }
                match str[1..].parse::<u32>() {
                    Ok(version) => {
                        if version > 0 {
                            found_version_segment = true;
                            Some(version)
                        } else {
                            return Err(eyre!("Invalid version {version}, lowest version is 1"));
                        }
                    }
                    Err(_) => {
                        return Err(eyre!(
                            "VERSION must be an integer in web name: '{dweb_host}"
                        ));
                    } // }
                }
            }
            None => {
                return Err(eyre!(
                    "Dweb host is missing DWEB-NAME and domain part: '{dweb_host}"
                ));
            }
        }
    } else {
        None
    };

    if segments.clone().count() != 3 {
        return Err(eyre!(
            "Dweb host must contain three or four segments, each separated by '.'"
        ));
    }

    // Next should be a DWEB-NAME
    let dweb_name = match segments.next() {
        Some(dweb_name) => dweb_name,
        None => {
            return Err(eyre!("Missing DWEB-NAME in '{dweb_host}"));
        }
    };

    match validate_dweb_name(&dweb_name) {
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    let mut ends_with_dlp_tld = false;
    if let Some(domain_part) = segments.next() {
        if domain_part == DOMAIN_PART {
            if let Some(tld_part) = segments.next() {
                if tld_part == TLD_PART && segments.next().is_none() {
                    ends_with_dlp_tld = true;
                }
            }
        }
    };
    if !ends_with_dlp_tld {
        return {
            Err(eyre!(
                "Dweb host does not end with '{DOMAIN_PART}.{TLD_PART} after the DWEB-NAME"
            ))
        };
    }

    #[cfg(feature = "fixed-dweb-hosts")]
    let is_fixed_dweb_host = !found_version_segment && dweb_name.contains(&fixed_dweb_host_tag);

    println!("DEBUG returning DwebHost: version: {version:?}, dweb_name: '{dweb_name}'");

    Ok(DwebHost {
        dweb_host_string: dweb_host.to_ascii_lowercase(),
        dweb_name: dweb_name.to_string().to_ascii_lowercase(),
        version,

        #[cfg(feature = "fixed-dweb-hosts")]
        is_fixed_dweb_host,
    })
}

/// Validate a DWEB-NAME string.
///
/// The part off a DWEB-NAME up but excluding the final hyphen is known as the 'memorable part'.
///
/// The memorable_part must start with at least two alphabetic characters. This is to allow it to
/// be distinguished from a version parameter, which is a 'v' or 'V' followed by an integer (u32),
/// which is useful in apps, for parsing links where the version is an optional part of the URL path.
///
/// Following the first two alphabetic characters are a number of alphanumeric characters which may
/// be separated by single hyphens, up to a total length for the memorable part of MEMORABLE_PART_LEN.
///
pub fn validate_dweb_name(dweb_name: &str) -> Result<()> {
    if dweb_name.len() < 2
        || !dweb_name.as_bytes()[0].is_ascii_alphabetic()
        || !dweb_name.as_bytes()[1].is_ascii_alphabetic()
    {
        return Err(eyre!(
            "DWEB-NAME must start with at least two alphabetic characters"
        ));
    }

    if !dweb_name[dweb_name.len() - 1..]
        .chars()
        .all(|c| c.is_alphanumeric())
    {
        return Err(eyre!("DWEB-NAME must end with an alphanumeric character"));
    }

    if !dweb_name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(eyre!(
            "DWEB-NAME can only contain letters, numbers (and non-consecutive hyphens)"
        ));
    }

    if dweb_name.contains("--") {
        return Err(eyre!("DWEB-NAME cannot contain '--'"));
    }

    Ok(())
}

/// Register a DWEB-NAME programmatically so it can be used in the browser address bar
pub async fn dwebname_register(dweb_name: &str, history_address: HistoryAddress) -> Result<()> {
    match validate_dweb_name(&dweb_name) {
        Ok(_) => (),
        Err(e) => {
            return Err(eyre!("Invalid DWEB-NAME '{dweb_name}' - {e}"));
        }
    };

    match &mut HISTORY_NAMES.lock() {
        Ok(lock) => {
            let cached_history_address = lock.get(dweb_name);
            if cached_history_address.is_some() {
                let cached_history_address = cached_history_address.unwrap();
                if history_address != *cached_history_address {
                    return Err(eyre!(
                        "DWEB-NAME '{dweb_name}' already in use for HISTORY-ADDRESS '{}'",
                        cached_history_address.to_hex()
                    ));
                }
                // println!("DWEB-NAME '{dweb_name}' already registered for {history_address_string}");
            } else {
                lock.insert(String::from(dweb_name), history_address);
                // println!(
                //     "DWEB-NAME '{dweb_name}' successfully registered for {history_address_string}"
                // );
            }
        }
        Err(e) => {
            return Err(eyre!("Failed to access dweb name cache - {e}"));
        }
    };

    Ok(())
}

#[test]
fn check_malformed_web_name() {
    use crate::web::name;
    assert!(name::decode_dweb_host("awe=some-f834.www-dweb.au").is_err());
    assert!(name::decode_dweb_host("awe@some-f834.www-dweb.au").is_err());
    assert!(name::decode_dweb_host("awe--some-f834.www-dweb.au").is_err());
    assert!(name::decode_dweb_host("awesom-f-e-f834.www-dweb.ant").is_err());
    assert!(name::decode_dweb_host("awesome-f834-.www-dweb.au").is_err());
    assert!(name::decode_dweb_host("awesome-f834.ww-dweb.au").is_err());
    assert!(name::decode_dweb_host("awesome-f834.www-dweb.ant").is_err());
    assert!(name::decode_dweb_host("awesome-f834.www-dweb.au.com").is_err());
    assert!(name::decode_dweb_host("v2.9awesome-f834.www-dweb.au").is_err());
    assert!(name::decode_dweb_host("v0.awesome-f834.www-dweb.au").is_err());
    assert!(name::decode_dweb_host("v2nd.awesome-f834.www-dweb.au").is_err());
}
