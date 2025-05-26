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

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use color_eyre::{eyre::eyre, Result};
use core::time::Duration;

use ant_logging::{LogFormat, LogOutputDest};
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::GraphEntryAddress;
use autonomi::PointerAddress;
use autonomi::ScratchpadAddress;

use dweb::helpers::convert::*;
use dweb::history::HistoryAddress;
use dweb::token::ShowCost;
use dweb::web::name::validate_dweb_name;

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
    /// Connect to the alpha public network
    #[clap(long)]
    pub alpha: bool,

    /// Connect to a local testnet
    #[clap(long, conflicts_with("alpha"))]
    pub local: bool,

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
    #[clap(long, value_parser = LogOutputDest::parse_from_str, verbatim_doc_comment, default_value = "stdout")]
    pub log_output_dest: LogOutputDest,

    /// Specify the network ID to use. This will allow you to run the CLI on a different network.
    ///
    /// By default, the network ID is set to 1, which represents the mainnet.
    #[clap(long, verbatim_doc_comment)]
    pub network_id: Option<u8>,

    /// Enable Autonomi network logging (to the terminal)
    #[clap(long, name = "client-logs", short = 'l', default_value = "false")]
    pub client_logs: bool,
    /// Override default and revert to earlier archive format (for publish commands)
    // Note: 'old' here means us PublicArchive rather than the default which is PrivateArchive, and that
    // this remains publicly accessible. Using PrivateArchive just means that the archive contains the datamaps
    // which saves a chunk per file in payment and when fetching.
    #[clap(long, default_value = "false")]
    pub use_old_archive: bool,
    // TODO implement remaining CLI options:
    // TODO --wallet-path <path-to-wallet-dir>
    /// Show the cost of dweb API calls after each call in tokens, gas, both or none
    #[clap(long, hide = true, default_value = "both")]
    pub show_dweb_costs: ShowCost,
    /// Optionally override the maximum fee in Gwei (units of 0.000000001 ETH), to pay for a transaction on the Arbitrum network.
    /// Useful if transaction fees are too high for the internal limit used in the Autonomi API.
    #[clap(long, hide = true, short = 'x')]
    pub max_fee_per_gas: Option<u128>,
    // Control API call retries (0 for unlimited tries)
    #[clap(long, hide = true, default_value = "0")]
    pub retry_api: u32,
    /// Do upload of directories one file at a time. Without this uploading a directory will start from scratch on each retry.
    /// When true, uploads may succeed more often but will cost more than if they are succeeding without retries.
    #[clap(long, hide = true, default_value = "true")]
    pub upload_file_by_file: bool,
    // Control API use of Pointers for versioned operations (e.g. for History and Registers).
    // By default it ignores Pointers as they aren't updating on mainnet.
    #[clap(long, hide = true, default_value = "true")]
    pub ignore_pointers: bool,
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
    /// Start a server to view Autonomi websites in your browser. Also required for some dweb subcommands.
    ///
    /// Afterwards, if you open a new terminal you can view a website in your browser
    /// by typing 'dweb open awesome'. The 'awesome' website contains links to other websites.
    ///
    /// See 'dweb open --help' to learn how to view other Autonomi websites.
    Serve {
        /// Experimental feature.
        ///
        /// Start a 'host' rather than 'port' based server. Requires a local DNS to be redirecting '*-dweb.au' to 127.0.0.1
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long = "experimental", default_value = "false")]
        experimental: bool,
        /// Optional host that will serve the request. Defaults to "127.0.0.1"
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "HOST", value_parser = parse_host)]
        host: Option<String>,
        /// The port that will serve the request (on localhost by default)
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "PORT", value_parser = parse_port_number)]
        port: Option<u16>,
    },

    /// Open a browser to view a website on Autonomi (requires 'dweb serve' running)
    ///
    /// You must have already started a server by typing 'dweb serve' in a different
    /// terminal before trying 'dweb open'.
    ///
    /// Example:
    ///
    /// dweb open awesome
    #[allow(non_camel_case_types)]
    Open {
        /// The website (or directory) you wish to open. This must be a HISTORY-ADDRESS, ARCHIVE-ADDRESS, a
        /// recognised DWEB-NAME or a DWEB-LINK. You can obtain addresses from others and if you publish
        /// your own website can share the address with others too.
        ///
        /// DWEB-NAME is a memorable name you can use to open a website like this: 'dweb open awesome'. You
        /// can add your own names using 'dweb name' or 'dweb open --as-name', but for now these will be
        /// forgotten if you re-start the server.
        ///
        /// HISTORY-ADDRESS is a long string which refers to a website history where you can view all versions
        /// of the website (using --version). It looks like this: a3e9f2dce9c441fbbb3fef505c77fd3069c8c51fd9890d4f8a073897f1f2d11254dabe2047ffdb28cf75444ee327557e
        ///
        /// ARCHIVE-ADDRESS is similar but shorter and refers to a single version.
        ///
        /// DWEB-LINK is used in websites when linking to other websites on Autonomi. For details see the
        /// documentation on github about building websites.
        ///
        /// TODO replace the above address with something interesting
        ///
        #[clap(value_name = "ADDRESS-NAME-OR-LINK")]
        address_name_or_link: String,
        /// The version of a History to select (if providing For a DWEB-NAME or HISTORY-ADDRESS).
        /// If not present the most recent version is assumed.
        #[clap(long, short = 'v', name = "VERSION")]
        version: Option<u32>,
        /// An optional remote path to open in the browser
        #[clap(name = "REMOTE-PATH")]
        remote_path: Option<String>,
        /// The host that will serve the request. Defaults to "127.0.0.1"
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "HOST", value_parser = parse_host)]
        host: Option<String>,
        /// The port that will serve the request (on localhost by default)
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "PORT", value_parser = parse_port_number)]
        port: Option<u16>,
        /// Use the 'with hosts' server rather than the ports based server. May need to use --port to select
        /// the relevant port for that server (e.g. --port 8081).
        ///
        /// Assumes the server was started with '--experimental' and that a local DNS has been set up.
        #[clap(hide = true, long, default_value = "false")]
        experimental: bool,
        /// Register a name for the website being opened
        #[clap(long = "as-name", short = 'a', value_name = "DWEB-NAME", value_parser = validate_dweb_name)]
        as_name: Option<String>,
    },

    /// Provide a memorable name for a directory or website (requires 'dweb serve' running)
    ///
    /// After setting a name you can use it in commands which accept DWEB-NAME
    /// as a parameter. For example, to open a website called 'myblog' type:
    ///
    /// dweb open myblog
    ///
    /// A DWEB-NAME is local to you so others cannot use it. It will last as
    /// long as the current server is running, but will be forgotten when it
    /// is shut down and restarted.
    Name {
        /// A short name (DWEB-NAME) for the site at ADDRESS-OR-NAME
        #[clap(value_name = "DWEB-NAME", value_parser = validate_dweb_name)]
        dweb_name: String,

        /// The address of a history on Autonomi
        #[clap(name = "HISTORY-ADDRESS", value_parser = str_to_history_address)]
        history_address: HistoryAddress,
        /// The host that will serve the request. Defaults to "127.0.0.1"
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "HOST", value_parser = parse_host)]
        host: Option<String>,
        /// The port that will serve the request (on localhost by default)
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "PORT", value_parser = parse_port_number)]
        port: Option<u16>,
        /// Use the 'with hosts' server rather than the ports based server. May need to use --port to select
        /// the relevant port for that server (e.g. --port 8081).
        ///
        /// Assumes the server was started with '--experimental' and that a local DNS has been set up.
        #[clap(hide = true, long, default_value = "false")]
        experimental: bool,
    },

    /// List all names currently recognised by the dweb server (requires 'dweb serve' running)
    #[allow(non_camel_case_types)]
    List_names {
        /// The host that will serve the request. Defaults to "127.0.0.1"
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "HOST", value_parser = parse_host)]
        host: Option<String>,
        /// The port that will serve the request (on localhost by default)
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "PORT", value_parser = parse_port_number)]
        port: Option<u16>,
        /// Use the 'with hosts' server rather than the ports based server. May need to use --port to select
        /// the relevant port for that server (e.g. --port 8081).
        ///
        /// Assumes the server was started with '--experimental' and that a local DNS has been set up.
        #[clap(hide = true, long, default_value = "false")]
        experimental: bool,
    },

    // TODO add an example or two to each command section
    /// Estimate the cost of publishing or updating a website
    Estimate {
        /// The root directory containing the website content to be published
        #[clap(long = "files-root", value_name = "FILES-ROOT")]
        files_root: PathBuf,
    },

    /// Publish a directory or website to Autonomi for the first time.
    ///
    /// Uploads a directory tree to Autonomi and pays using the default wallet.
    ///
    /// If successful, prints the address of the directory history which
    /// can be viewed using a web browser when a dweb server is running.
    #[allow(non_camel_case_types)]
    Publish_new {
        /// The root directory of the content to be published
        #[clap(long = "files-root", value_name = "FILES-ROOT")]
        files_root: PathBuf,
        /// Associate NAME with the directory history, for use with 'dweb publish-update'.
        /// Defaults to the name of the website directory (FILES-ROOT).
        #[clap(long, short = 'n')]
        name: Option<String>,
        /// Optional configuration when uploading content for a website. This can specify alternative
        /// default index file(s), redirects etc.
        /// You can either specify a path here or include the settings in <FILES-ROOT>/.dweb/dweb-settings.json
        #[clap(long = "dweb-settings", short = 'c', value_name = "JSON-FILE")]
        dweb_settings: Option<PathBuf>,
        /// Disable the AWV check when publishing a new website to allow for init of a new Autonomi network (during beta)
        #[clap(long, name = "is-new-network", hide = true, default_value = "false")]
        is_new_network: bool,
    },

    /// Update a previously published directory or website and preserve older versions on Autonomi.
    ///
    /// Uploads changed files and makes this the default version. Pays using the default wallet.
    ///
    /// If successful, prints the address of the directory history which
    /// can be viewed using a web browser when a dweb server is running.
    #[allow(non_camel_case_types)]
    Publish_update {
        /// The root directory containing the new website content to be uploaded
        #[clap(long = "files-root", value_name = "FILES-ROOT")]
        files_root: PathBuf,
        /// The NAME used when the website was first published.
        /// Defaults to use the name of the website directory (FILES-ROOT)
        #[clap(long, short = 'n')]
        name: Option<String>,
        /// Optional configuration when uploading content for a website. This can specify alternative
        /// default index file(s), redirects etc.
        /// You can either specify a path here or include the settings in <FILES-ROOT>/.dweb/dweb-settings.json
        #[clap(long = "dweb-settings", short = 'c', value_name = "JSON-FILE")]
        dweb_settings: Option<PathBuf>,
    },

    /// Download a file or directory. TODO: not yet implemented
    #[clap(hide = true)] // TODO hide until implemented
    Download {
        /// This needs revising for dweb to access different address types using --history-address, --directory-address, --file-address
        ///
        /// For a history, you must provide the RANGE of entries to be processed.
        ///
        /// For a directory you may specify the path of a specific file or directory to be downloaded
        /// by including this at the end of the ARCHIVE-ADDRESS. This defaults to the directory root ('/').
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

    #[allow(non_camel_case_types)]
    Wallet_info {},

    /// Print information about a history of data stored on Autonomi.
    #[allow(non_camel_case_types)]
    Inspect_history {
        /// The address of a History on Autonomi
        /// TODO add ability to query the server so this can be HISTORY-ADDRESS-OR-NAME
        /// TODO note that to use recognised names (DWEB-NAME) the server must be running
        #[clap(name = "HISTORY-ADDRESS")]
        address_or_name: String,

        /// Print a summary of the History including type (the value of entry 0) and number of entries
        #[clap(long = "full", short = 'f', default_value = "false")]
        print_history_full: bool,

        /// Print information about each entry in RANGE, which can be
        /// an integer (for a single entry), or an integer followed by ':' or
        /// two integers separated by ':'. The first entry is position 0
        /// and the last is 'size minus 1'
        #[clap(long = "entries", short = 'e', value_name = "RANGE", value_parser = str_to_entries_range )]
        entries_range: Option<EntriesRange>,

        /// Shorten GraphEntry hex strings to the first six characters plus '..'
        #[clap(long = "brief", short = 'b', default_value = "false")]
        shorten_hex_strings: bool,

        /// For each entry in RANGE print information about files stored on
        /// the network, as recorded in the directory pointed to by the entry. Enables
        /// the following 'print' options for files metadata entries in RANGE
        #[clap(
            long = "include-files",
            default_value = "false",
            requires = "entries_range",
            conflicts_with("graph_keys")
        )]
        include_files: bool,

        /// Show the public keys in a GraphEntry rather than the addresses
        /// of parent/descendents in the entry. Default is to show the
        /// addresses.
        #[clap(long = "graph-with-keys", short = 'k', default_value = "false")]
        graph_keys: bool,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Print information about a GraphEntry stored on Autonomi.
    ///
    /// Note: descendents are shown as public keys rather than addresses. This is for
    /// two reasons. Firstly this is what is stored in the GraphEntry, and secondly
    /// they cannot be converted to addresses without the main public key of the History
    /// or Register which created them. I assume this is to prevent someone finding a
    /// GraphEntry and then following the graph without having the public key of
    /// the History or Register. If you wish to follow the graph, see the inspect-history
    /// command.
    // TODO: [ ] inspect-graph --root-address|--history-address|--pointer-address
    #[allow(non_camel_case_types)]
    Inspect_graphentry {
        /// The address of a GraphEntry on Autonomi
        #[clap(name = "GRAPHENTRY-ADDRESS", value_parser = str_to_graph_entry_address)]
        graph_entry_address: GraphEntryAddress,

        /// Print full details of GraphEntry
        #[clap(long = "full", short = 'f', default_value = "false")]
        print_full: bool,

        /// Shorten long hex strings to the first six characters plus '..'
        #[clap(long = "brief", short = 'b', default_value = "false")]
        shorten_hex_strings: bool,
    },

    /// Print information about a Pointer stored on Autonomi
    #[allow(non_camel_case_types)]
    Inspect_pointer {
        /// The address of a Pointer on Autonomi
        #[clap(name = "POINTER-ADDRESS", value_parser = str_to_pointer_address)]
        pointer_address: PointerAddress,
    },

    /// Print information about a Scratchpad stored on Autonomi
    #[allow(non_camel_case_types)]
    Inspect_scratchpad {
        /// The address of a Scratchpad on Autonomi
        #[clap(name = "SCRATCHPAD-ADDRESS", value_parser = str_to_scratchpad_address)]
        scratchpad_address: ScratchpadAddress,
        /// Attempt to show the data as text. Only works for unencrypted Scratchpads
        #[clap(long = "", short = 't', default_value = "false")]
        data_as_text: bool,
    },

    /// Print information about files in a directory on Autonomi
    #[allow(non_camel_case_types)]
    Inspect_files {
        /// The address of some a directory uploaded to Autonomi
        #[clap(value_name = "ARCHIVE-ADDRESS", value_parser = str_to_archive_address)]
        archive_address: ArchiveAddress,

        #[command(flatten)]
        files_args: FilesArgs,
    },

    /// Access OpenAPI documentation for dweb and Autonomi RESTful APIs (requires 'dweb serve' running)
    ///
    /// By default opens a website built into dweb showing the RESTful APIs
    /// supported by dweb. This uses the Swagger UI.
    ///
    /// Alternatively can print the OpenAPI specified APIs
    /// to the terminal in JSON format.
    #[allow(non_camel_case_types)]
    Openapi_docs {
        /// Print dweb APIs to the terminal as OpenAPI JSON
        #[clap(long = "print", default_value = "false")]
        print: bool,
        /// The host that will serve the request. Defaults to "127.0.0.1"
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "HOST", value_parser = parse_host)]
        host: Option<String>,
        /// The port that will serve the request (on localhost by default)
        /// This is only needed when not using defaults, so hidden to de-clutter the CLI help
        #[clap(hide = true, long, value_name = "PORT", value_parser = parse_port_number)]
        port: Option<u16>,
    },
}

#[derive(Args, Debug)]
pub struct FilesArgs {
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
    pub start: Option<u32>,
    pub end: Option<u32>,
}

fn str_to_entries_range(s: &str) -> Result<EntriesRange> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d*)(:?)(\d*)$").unwrap());

    let captures = match RE.captures(s) {
        Some(captures) => captures,
        None => return Err(eyre!("invalid range")),
    };

    let start = if !captures[1].is_empty() {
        match captures[1].parse::<u32>() {
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
            match captures[3].parse::<u32>() {
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
