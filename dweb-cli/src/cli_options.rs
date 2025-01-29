/*
Copyright (c) 2024-2025 Mark Hughes

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, ord
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/
use std::path::PathBuf;
use std::sync::LazyLock;

use ant_protocol::storage::PointerAddress as HistoryAddress;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use color_eyre::{eyre::eyre, Result};
use core::time::Duration;
use xor_name::XorName;

use ant_bootstrap::PeersArgs;
use ant_logging::{LogFormat, LogOutputDest};

use dweb::helpers::convert::*;

// TODO add example to each CLI subcommand

///! Command line options and usage
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "a web publishing and browsing app for Autonomi peer-to-peer network"
)]
pub struct Opt {
    /// Optional awe URL to browse.
    ///
    /// Use awv://<HISTORY-ADDRESS> to browse most recent version from the history. (Use --history-version to specify a version).
    ///
    /// Use awm://<DIRECTORY-ADDRESS> to browse files or website from DirectoryTree
    ///
    /// Use awf://<FILE-ADDRESS> to load or fetch to a file rather than a website.
    pub url: Option<String>,

    /// Browse the specified version from the history
    #[clap(long, value_parser = greater_than_0)]
    pub history_version: Option<u64>,

    #[command(flatten)]
    pub peers: PeersArgs,

    /// Available sub commands
    #[command(subcommand)]
    pub cmd: Option<Subcommands>,

    /// The maximum duration to wait for a connection to the network before timing out.
    #[clap(long = "timeout", value_parser = |t: &str| -> Result<Duration> { Ok(t.parse().map(Duration::from_secs)?) })]
    pub connection_timeout: Option<Duration>,

    /// Specify the logging format.
    ///
    /// Valid values are "default" or "json".
    ///
    /// If the argument is not used, the default format will be applied.
    #[clap(long, value_parser = LogFormat::parse_from_str, verbatim_doc_comment)]
    pub log_format: Option<LogFormat>,

    /// Specify the logging output destination.
    ///
    /// Valid values are "stdout", "data-dir", or a custom path.
    ///
    /// `data-dir` is the default value.
    ///
    /// The data directory location is platform specific:
    ///  - Linux: $HOME/.local/share/autonomi/client/logs
    ///  - macOS: $HOME/Library/Application Support/autonomi/client/logs
    ///  - Windows: C:\Users\<username>\AppData\Roaming\autonomi\client\logs
    #[allow(rustdoc::invalid_html_tags)]
    #[clap(long, value_parser = LogOutputDest::parse_from_str, verbatim_doc_comment, default_value = "data-dir")]
    pub log_output_dest: LogOutputDest,

    /// Specify the network ID to use. This will allow you to run the CLI on a different network.
    ///
    /// By default, the network ID is set to 1, which represents the mainnet.
    #[clap(long, verbatim_doc_comment)]
    pub network_id: Option<u8>,

    /// Enable Autonomi network logging (to the terminal)
    #[clap(long, name = "client-logs", short = 'l', default_value = "false")]
    pub client_logs: bool,
    // TODO remove in favour of WebCmds subcommand
    // /// Local path of static HTML files to publish
    // #[clap(long = "publish-website")]
    // pub files_root: Option<PathBuf>,
    // TODO implement remaining CLI options:
    // TODO --wallet-path <path-to-wallet-dir>
}

fn greater_than_0(s: &str) -> Result<u64, String> {
    match s.parse::<u64>() {
        Err(e) => Err(e.to_string()),
        Ok(value) => {
            if value >= 1 {
                Ok(value)
            } else {
                Err(String::from("Number must be greater than zero"))
            }
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Subcommands {
    // TODO add an example or two to each command section
    /// Estimate the cost of publishing or updating a website
    Estimate {
        /// The root directory containing the website content to be published
        #[clap(long = "files-root", value_name = "FILES-ROOT")]
        files_root: PathBuf,
    },

    /// Publish a new website
    ///
    /// Uploads a directory tree of content to Autonomi and pays using the default wallet
    ///
    /// If successful, prints the xor address of the directory history, accessible
    /// using Awe Browser using a URL like 'awv://<HISTORY-ADDRESS>'.
    #[allow(non_camel_case_types)]
    Publish_new {
        /// The root directory of the content to be published
        #[clap(long = "files-root", value_name = "FILES-ROOT")]
        files_root: PathBuf,
        /// Publish a website and associate it with this name
        /// Defaults to the name of the website directory (FILES-ROOT)
        #[clap(long, short = 'n')]
        name: Option<String>,
        /// Optional configuration when uploading content for a website, such as default index file(s), redirects etc.
        /// TODO review implementation when dweb-lib implements config via a file in the website directory
        #[clap(long = "website-config", short = 'c', value_name = "JSON-FILE")]
        website_config: Option<PathBuf>,
        //
        /// Disable the AWV check when publishing a new website to allow for init of a new Autonomi network (during beta)
        #[clap(long, name = "is-new-network", hide = true, default_value = "false")]
        is_new_network: bool,
    },

    /// Update a previously uploaded directory while preserving old versions on Autonomi
    ///
    /// Uploads changes in the directory content and makes this the
    /// default version. Pays using the default wallet.
    ///
    /// If successful, prints the xor address of the content, accessible
    /// using Awe Browser using a URL like 'awv://HISTORY-ADDRESS'.
    #[allow(non_camel_case_types)]
    Publish_update {
        /// The root directory containing the new website content to be uploaded
        #[clap(long = "files-root", value_name = "FILES-ROOT")]
        files_root: PathBuf,
        /// The name used when the website was first published
        /// Defaults to use the name of the website directory (FILES-ROOT)
        #[clap(long, short = 'n', conflicts_with = "history_address")]
        name: Option<String>,
        /// Optional configuration when uploading content for a website, such as default index file(s), redirects etc.
        /// TODO review implementation when dweb-lib implements config via a file in the website directory
        #[clap(long = "website-config", short = 'c', value_name = "JSON-FILE")]
        website_config: Option<PathBuf>,
    },

    /// Download a file or directory. TODO: not yet implemented
    #[clap(hide = true)] // TODO hide until implemented
    Download {
        /// This needs revising for dweb to access different address types using --history-address, --directory-address, --file-address
        ///
        /// For a history, you must provide the RANGE of entries to be processed.
        ///
        /// For a directory you may specify the path of a specific file or directory to be downloaded
        /// by including this at the end of the DIRECTORY-ADDRESS. This defaults to the directory root ('/').
        ///
        /// If you do not specify a DOWNLOAD-PATH the content downloaded will be printed
        /// on the terminal (via stdout).
        // TODO implement a parser so I can validate here any combo of protocols (but keep as String here)
        #[clap(value_name = "AWE-URL")]
        awe_url: String,

        /// A file or directory path where downloaded data is to be stored. This must not exist.
        /// If downloading more than a single file, DOWNLOAD-PATH must end with a file separator, and
        /// a directory will be created to hold the downloaded files and any subdirectories.
        #[clap(value_name = "DOWNLOAD-PATH")]
        /// TODO: PathBuf?
        filesystem_path: Option<String>,

        /// When providing a HISTORY-ADDRESS you must specify the entry or
        /// entries you with to download with this option. The download will be applied for each
        /// entry in RANGE, which can be an integer (for a single entry), or an integer followed
        /// by ':' or two integers separated by ':'. The first entry is position 0 and the last is
        /// history 'size minus 1'. When more than one entry is downloaded, each will be saved in
        /// a separate subdirectory of the <DOWNLOAD-PATH>, named with a 'v' followed by the index
        /// of the entry, such as 'v3', 'v4' etc.
        #[clap(long = "entries", short = 'e', value_name = "RANGE", value_parser = str_to_entries_range)]
        entries_range: Option<EntriesRange>,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Print information about data structures stored on Autonomi
    #[allow(non_camel_case_types)]
    Inspect_history {
        /// The address of a history on Autonomi
        #[clap(name = "HISTORY-ADDRESS", value_parser = str_to_pointer_address)]
        history_address: HistoryAddress,

        /// Print a summary of the history including type (the value of entry 0) and number of entries
        #[clap(long = "history-summary", short = 'r', default_value = "false")]
        print_history_summary: bool,

        /// Print the type of data recorded in the history (the value of entry 0)
        #[clap(long = "type", short = 't', default_value = "false")]
        print_type: bool,

        /// Print the number of entries
        #[clap(long = "size", short = 's', default_value = "false")]
        print_size: bool,

        /// Print information about each entry in RANGE, which can be
        /// an integer (for a single entry), or an integer followed by ':' or
        /// two integers separated by ':'. The first entry is position 0
        /// and the last is 'size minus 1'
        #[clap(long = "entries", short = 'e', value_name = "RANGE", value_parser = str_to_entries_range )]
        entries_range: Option<EntriesRange>,

        /// For each entry in RANGE print information about files stored on
        /// the network, as recorded by the metadata pointed to by the entry. Enables
        /// the following 'print' options for files metadata entries in RANGE
        #[clap(
            long = "include-files",
            default_value = "false",
            requires = "entries_range"
        )]
        include_files: bool,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Print information about files in a directory on Autonomi
    #[allow(non_camel_case_types)]
    Inspect_files {
        /// The address of some a directory uploaded to Autonomi
        #[clap(value_name = "DIRECTORY-ADDRESS", value_parser = str_to_xor_name)]
        directory_address: XorName,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Open a browser to view a website on Autonomi - not yet implemented
    #[clap(hide = true)] // TODO hide until dweb::web::handle_open_browser() is implemented
    Browse {
        /// A name which will be registered for this session as part of the URL hostname for this site
        #[clap(value_name = "DWEB-NAME", value_parser = dweb::web::name::validate_dweb_name)]
        dweb_name: String,

        /// The address of a history on Autonomi
        #[clap(name = "HISTORY-ADDRESS", value_parser = str_to_pointer_address, conflicts_with("directory_address"))]
        history_address: Option<HistoryAddress>,
        // /// The address of some a directory uploaded to Autonomi
        // #[clap(value_name = "DIRECTORY-ADDRESS", value_parser = str_to_xor_name)]
        // directory_address: Option<XorName>,  only if I support feature("fixed-dweb-hosts")
    },

    /// Start a server to allow you to view Autonomi websites in any web browser
    Serve {
        /// Optional port number on which to listen for local requests
        #[clap(value_name = "PORT", default_value = DEFAULT_HTTP_PORT_STR, value_parser = parse_port_number)]
        port: u16,

        /// Optional host on which to listen for local requests
        #[clap(value_name = "HOST", default_value = LOCALHOST, value_parser = parse_host)]
        host: String,
    },
}

#[derive(Args, Debug)]
pub struct FilesArgs {
    /// Print summary information about files based on files metadata
    #[clap(long = "metadata-summary", short = 'm', default_value = "false")]
    pub print_metadata_summary: bool,

    /// Print the number of directories and files
    #[clap(long = "count", short = 'c', default_value = "false")]
    pub print_counts: bool,

    /// Print the total number of bytes for all files
    #[clap(long = "total-bytes", short = 'b', default_value = "false")]
    pub print_total_bytes: bool,

    /// Print the path of each file
    #[clap(long = "paths", short = 'p', default_value = "false")]
    pub print_paths: bool,

    /// Print metadata about each file including path, modification time and size in bytes
    #[clap(long = "details", short = 'd', default_value = "false")]
    pub print_all_details: bool,
}

use regex::Regex;
#[derive(Clone, Debug)]
pub struct EntriesRange {
    pub start: Option<usize>,
    pub end: Option<usize>,
}

fn str_to_entries_range(s: &str) -> Result<EntriesRange> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d*)(:?)(\d*)$").unwrap());

    let captures = match RE.captures(s) {
        Some(captures) => captures,
        None => return Err(eyre!("invalid range")),
    };

    let start = if !captures[1].is_empty() {
        match captures[1].parse::<usize>() {
            Ok(n) => Some(n),
            Err(_) => return Err(eyre!("invalid start value")),
        }
    } else {
        None
    };

    let end = if start.is_some() && captures[2].is_empty() {
        start
    } else {
        if !captures[3].is_empty() {
            match captures[3].parse::<usize>() {
                Ok(n) => Some(n),
                Err(_) => return Err(eyre!("invalid end value")),
            }
        } else {
            None
        }
    };

    if let (Some(start), Some(end)) = (start, end) {
        if end < start {
            return Err(eyre!("end cannot be less than start"));
        }
    }

    Ok(EntriesRange { start, end })
}

// pub fn get_app_name() -> String {
//     String::from(???)
// }

// pub fn get_app_version() -> String {
//     String::from(structopt::clap::crate_version!())
// }
