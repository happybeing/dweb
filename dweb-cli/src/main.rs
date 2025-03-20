/*
*   Copyright (c) 2024-2025 Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU Affero General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU Affero General Public License for more details.

*   You should have received a copy of the GNU Affero General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#[macro_use]
extern crate tracing;

mod cli_options;
mod commands;
mod experimental;
mod generated_rs;
mod helpers;
mod services;

use clap::Parser;
use color_eyre::Result;
use tracing::Level;

// #[cfg(feature = "metrics")]
// use ant_logging::metrics::init_metrics;
use ant_logging::{LogBuilder, LogFormat};

use crate::commands::subcommands;
use cli_options::Opt;

#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install().expect("Failed to initialise error handler");
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }

    let opt = Opt::parse();
    if let Some(network_id) = opt.network_id {
        ant_protocol::version::set_network_id(network_id);
    }

    // TODO Keep up-to-date with autonomi/ant-cli/src/main.rs init_logging_and_metrics()
    let _gaurds;
    if opt.client_logs {
        println!("DEBUG default logging targets enabled");
        let logging_targets = vec![
            ("ant_bootstrap".to_string(), Level::DEBUG),
            ("ant_build_info".to_string(), Level::TRACE),
            ("ant_evm".to_string(), Level::TRACE),
            ("ant_logging".to_string(), Level::TRACE),
            ("ant_networking".to_string(), Level::INFO),
            ("ant_registers".to_string(), Level::TRACE),
            ("evmlib".to_string(), Level::TRACE),
            ("autonomi_cli".to_string(), Level::TRACE),
            ("autonomi".to_string(), Level::TRACE),
        ];

        let mut log_builder = LogBuilder::new(logging_targets);
        log_builder.output_dest(opt.log_output_dest.clone());
        log_builder.format(opt.log_format.unwrap_or(LogFormat::Default));
        _gaurds = log_builder.initialize().unwrap();
    }

    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }

    subcommands::cli_commands(opt).await?;

    Ok(())
}
