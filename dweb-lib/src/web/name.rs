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

//! # WebName
//!
//! The 'host' part of a dweb web URL uses the domain 'www-dweb.au' plus either
//! one or two subdomains:
//!
//!    `[v<VERSION>.]<SHORTNAME>.www-dweb.au`
//!
//! The first part is an optional followed by a short name (which correponds to a History
//! stored on Autonomi).
//!
//!    VERSION 1 is the first version, 2 the second etc, and if omitted implies 'most recent'.
//!
//!    A SHORTNAME corresponds to a particular website history. It begins with a memorable part,
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
//! WebNames allow the correct website version to be retrieved from a History<DirectoryTree>
//! on Autonomi and the corresponding content to be returned to a standard web browser. They act
//! as keys for a cache maintained by the local dweb server, but must first be created using
//! the appropriate dweb APIs.
//!
//! Once created, resolving a WebName requires a local DNS to redirect the dweb domain www-dweb.au
//! to a local dweb server (e.g. dweb-cli) which decodes the name and accesses the relevant website
//! version from a cache held in the server.
//!
//! WebNames could be persisted in various ways, such as in a separate website on Autonomi or
//! the private Vault of a user, which then provides a set of 'favourites' or web bookmarks personal
//! to a user.
//!
//! Without persistence, different SHORTNAMES can be used with the same HistoryAddress at different
//! times, there is always a one-to-one correspondence between the two, so neither can be coupled
//! to more than one of the other at one time.
//!
//! TODO: implement persistent SHORTNAMES per user and use to provide a page of sites with brief
//! information to aid identification.
//!

use color_eyre::eyre::{eyre, Result};

use ant_registers::RegisterAddress as HistoryAddress;
use xor_name::XorName as DirectoryAddress;

// Domain name and subdomain constraints based on IETF RFC1035 with links to relevant sections:
pub const MAX_SUBDOMAIN_LEN: usize = 63; //  S2.3.4 Size limits (https://datatracker.ietf.org/doc/html/rfc1035#section-2.3.4)
                                         // A subdomain must start with a letter (a-z) and is followed by one or more letters or numbers
                                         // which may be separated by a hyphen. S2.3.1. Preferred name syntax (https://datatracker.ietf.org/doc/html/rfc1035#section-2.3.1)

pub const DISAMBIGUATION_LEN: usize = 4; // Number of hexadecimal disambiguation characters to include in a SHORTNAME
pub const MEMORABLE_PART_LEN: usize = MAX_SUBDOMAIN_LEN - DISAMBIGUATION_LEN - 1; // Allow 1 for hyphen

pub const DOMAIN_PART: &str = "www-dweb";
pub const TLD_PART: &str = "au";

const VERSION_CHAR: u8 = b'v';
const FIXED_WEBNAME_SEPARATOR: &str = "-f";

/// WebName encapsulates a web name string of a dweb URL, and its component subdomain and domain
/// parts which can be used to lookup the content for a version of a website.
///
pub struct WebName {
    /// `v[<VERSION>].<SHORTNAME>.www-dweb.au`
    pub web_name_string: String,
    pub shortname: String,
    /// None implies most recent version (highest number)
    pub version: Option<u64>,

    #[feature("fixed-webnames")]
    // Development build feature for non-versioned DirectoryTree references
    pub is_fixed_webname: bool,
}

/// Make a valid SHORTNAME for a dweb URL
///
/// memorable_part must start with an alphabetic character followed by zero or more alphanumeric
/// characters which may be separated by single hyphens, up to a length of MEMORABLE_PART_LEN.
pub fn make_shortname(memorable_part: &String, history_address: HistoryAddress) -> Result<String> {
    if memorable_part.len() == 0 {
        return Err(eyre!(
            "SHORTNAME memorable must include at least one alphabetic character"
        ));
    }

    if !memorable_part.as_bytes()[0].is_ascii_alphabetic() {
        return Err(eyre!(
            "SHORTNAME memorable part must begin with an alphabetic character"
        ));
    }

    if !memorable_part.len() > MEMORABLE_PART_LEN {
        return Err(eyre!(
            "SHORTNAME memorable part cannot exceed {MEMORABLE_PART_LEN} characters"
        ));
    }

    if memorable_part.contains("--") {
        return Err(eyre!(
            "SHORTNAME memorable part cannot contain consecutive hyphens"
        ));
    }

    if !memorable_part[1..]
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(eyre!(
            "SHORTNAME memorable part can only contain alphanumeric characters and hyphens"
        ));
    }

    // Prevent clash with 'fixed version' web names
    if !memorable_part.ends_with(FIXED_WEBNAME_SEPARATOR) {
        return Err(eyre!(
            "SHORTNAME memorable part cannot end with '{FIXED_WEBNAME_SEPARATOR}'"
        ));
    }

    let history_part = format!("{history_address}");
    Ok(
        memorable_part[..MEMORABLE_PART_LEN].to_string()
            + "-"
            + &history_part[..DISAMBIGUATION_LEN],
    )
}

/// Create a version part ("v[VERSION]") for a www-dweb URL
pub fn make_version_part(version: u64) -> String {
    if version > 0 {
        format!("v{version}")
    } else {
        String::from("")
    }
}

#[feature("fixed-webnames")]
pub fn make_fixed_shortname(
    memorable_part: &String,
    directory_address: DirectoryAddress,
) -> Result<String> {
    if memorable_part.len() == 0 {
        return Err(eyre!(
            "SHORTNAME memorable must include at least one alphabetic character"
        ));
    }

    if !memorable_part.as_bytes()[0].is_ascii_alphabetic() {
        return Err(eyre!(
            "SHORTNAME memorable part must begin with an alphabetic character"
        ));
    }

    const FIXED_MEMORABLE_PART_LEN: usize = MEMORABLE_PART_LEN - FIXED_WEBNAME_SEPARATOR.len();
    if !memorable_part.len() > FIXED_MEMORABLE_PART_LEN {
        return Err(eyre!(
            "'fixed' version website SHORTNAME memorable part cannot exceed {FIXED_MEMORABLE_PART_LEN} characters"
        ));
    }

    if memorable_part.contains("--") {
        return Err(eyre!(
            "SHORTNAME memorable part cannot contain consecutive hyphens"
        ));
    }

    if !memorable_part[1..]
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(eyre!(
            "SHORTNAME memorable part can only contain alphanumeric characters and hyphens"
        ));
    }

    // Prevent clash with 'fixed version' web names
    if !memorable_part.ends_with(FIXED_WEBNAME_SEPARATOR) {
        return Err(eyre!(
            "SHORTNAME memorable part cannot end with '{FIXED_WEBNAME_SEPARATOR}'"
        ));
    }

    let directory_part = format!("{directory_address:64x}");
    let web_name = memorable_part[..MEMORABLE_PART_LEN].to_string()
        + FIXED_WEBNAME_SEPARATOR
        + "-"
        + &directory_part[..];

    Ok(web_name.to_ascii_lowercase())
}

/// Decode a web name string
/// Returns a WebName which includes the validated web name string, SHORTNAME and VERSION (if present)
///
///  For example, 'v2.awesome-f834.www-dweb.au' would return
///     Ok(WebName{
///         web_name_string: &"v2.awesome-f834.www-dweb.au",
///         short_name: &"awesome-f834",
///         version: Some(2)
///     })
///
/// # Examples
///
/// ```
/// use crate::dweb::web::name;
/// assert!(name::decode_web_name("v2.awesome-f834.www-dweb.au").is_ok());
/// assert!(name::decode_web_name("awesome-f834.www-dweb.au").is_ok());
/// assert!(name::decode_web_name("awesome99-f834.www-dweb.au").is_ok());
/// assert!(name::decode_web_name("awe-some-f834.www-dweb.au").is_ok());
/// assert!(name::decode_web_name("awe-99some-f834.www-dweb.au").is_ok());
///
/// assert!(name::decode_web_name("9awesome-f834.www-dweb.au").is_err());
/// assert!(name::decode_web_name("awe=some-f834.www-dweb.au").is_err());
/// assert!(name::decode_web_name("awe--some-f834.www-dweb.au").is_err());
/// assert!(name::decode_web_name(&String::from("v.awesome-f834.www-dweb.au").as_str()).is_err());
///
/// ```
//
// Note: for --features=fixed-webnames, this will also decode fixed web names which are
// differentiated by a SHORTNAME containing the String::from(FIXED_WEBNAME_SEPARATOR) + "-";
pub fn decode_web_name(web_name: &str) -> Result<WebName> {
    println!("DEBUG: decode_web_name({web_name})...");
    if web_name.len() == 0 {
        return Err(eyre!("Web name cannot be zero length"));
    }

    let fixed_web_name_tag = String::from(FIXED_WEBNAME_SEPARATOR) + "-";

    let mut segments = web_name.split('.');
    let total_segments = segments.clone().count();
    if total_segments > 4 || total_segments < 3 {
        return Err(eyre!(
            "Web name must contain three or four segments, each separated by '.'"
        ));
    }

    let mut found_version_segment = false;
    // If four segments are present, process the first as 'v<VERSION>'
    let version = if segments.clone().count() == 4 && web_name.as_bytes()[0] == VERSION_CHAR {
        match segments.next() {
            Some(str) => {
                if !str.starts_with('v') {
                    return Err(eyre!(
                        "Web name contains four segments (separated by '.') so first must start with 'v'"
                    ));
                }
                match str[1..].parse::<u64>() {
                    Ok(version) => {
                        if version > 0 {
                            found_version_segment = true;
                            Some(version)
                        } else {
                            return Err(eyre!("Invalid version {version}, lowest version is 1"));
                        }
                    }
                    Err(_) => {
                        return Err(eyre!("VERSION must be an integer in web name: '{web_name}"));
                    } // }
                }
            }
            None => {
                return Err(eyre!(
                    "Web name is missing SHORTNAME and domain part: '{web_name}"
                ));
            }
        }
    } else {
        None
    };

    if segments.clone().count() != 3 {
        return Err(eyre!(
            "Web name must contain three or four segments, each separated by '.'"
        ));
    }

    let shortname = match segments.next() {
        Some(shortname) => shortname,
        None => {
            return Err(eyre!("Missing SHORTNAME in '{web_name}"));
        }
    };

    if !shortname.as_bytes()[0].is_ascii_alphabetic() {
        return Err(eyre!(
            "Web name SHORTNAME must start with an alphbetic character"
        ));
    }

    if !shortname[shortname.len() - 1..]
        .chars()
        .all(|c| c.is_alphanumeric())
    {
        return Err(eyre!(
            "Web name SHORTNAME must end with an alphanumeric character"
        ));
    }

    if !shortname.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(eyre!(
            "Web name SHORTNAME can only contain letters, numbers (and non-consecutive hyphens)"
        ));
    }

    if shortname.contains("--") {
        return Err(eyre!("Web name SHORTNAME cannot contain '--'"));
    }

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
                "Web name does not end with '{DOMAIN_PART}.{TLD_PART} after the SHORTNAME"
            ))
        };
    }

    #[feature("fixed-webnames")]
    let is_fixed_webname = !found_version_segment && shortname.contains(&fixed_web_name_tag);

    println!("DEBUG: returning WebName: version: {version:?}, shortname: '{shortname}'");

    Ok(WebName {
        web_name_string: web_name.to_ascii_lowercase(),
        shortname: shortname.to_string().to_ascii_lowercase(),
        version,

        #[feature("fixed-webnames")]
        is_fixed_webname,
    })
}

#[test]
fn check_malformed_web_name() {
    use crate::web::name;
    assert!(name::decode_web_name("awe=some-f834.www-dweb.au").is_err());
    assert!(name::decode_web_name("awe@some-f834.www-dweb.au").is_err());
    assert!(name::decode_web_name("awe--some-f834.www-dweb.au").is_err());
    assert!(name::decode_web_name("awesom-f-e-f834.www-dweb.ant").is_err());
    assert!(name::decode_web_name("awesome-f834-.www-dweb.au").is_err());
    assert!(name::decode_web_name("awesome-f834.ww-dweb.au").is_err());
    assert!(name::decode_web_name("awesome-f834.www-dweb.ant").is_err());
    assert!(name::decode_web_name("awesome-f834.www-dweb.au.com").is_err());
    assert!(name::decode_web_name("v2.9awesome-f834.www-dweb.au").is_err());
    assert!(name::decode_web_name("v0.awesome-f834.www-dweb.au").is_err());
    assert!(name::decode_web_name("v2nd.awesome-f834.www-dweb.au").is_err());
}
