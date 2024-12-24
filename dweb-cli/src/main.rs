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
mod connect;
mod generated_rs;
mod helpers;
mod serve;

use clap::Parser;
use color_eyre::Result;
use tracing::Level;

// #[cfg(feature = "metrics")]
// use ant_logging::metrics::init_metrics;
use ant_logging::{LogBuilder, LogFormat, ReloadHandle, WorkerGuard};

use crate::commands::awe_subcommands;
use cli_options::Opt;

#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install().expect("Failed to initialise error handler");

    let opt = Opt::parse();
    let _result_log_guards = init_logging_and_metrics(&opt);

    // Log the full command that was run and the git version
    info!("\"{}\"", std::env::args().collect::<Vec<_>>().join(" "));
    let version = ant_build_info::git_info();
    info!("dweb built with autonomi git version: {version}");
    println!("dweb built with autonomi git version: {version}");

    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }

    // TODO temp hack until awe_subcommands is stable
    serve::serve(8080).await?;
    // awe_subcommands::cli_commands(opt).await?;

    Ok(())
}

fn init_logging_and_metrics(opt: &Opt) -> Result<(ReloadHandle, Option<WorkerGuard>)> {
    let logging_targets = vec![
        ("ant_bootstrap".to_string(), Level::DEBUG),
        ("ant_build_info".to_string(), Level::TRACE),
        ("ant_evm".to_string(), Level::TRACE),
        ("ant_networking".to_string(), Level::INFO),
        ("ant_registers".to_string(), Level::TRACE),
        ("autonomi_cli".to_string(), Level::TRACE),
        ("autonomi".to_string(), Level::TRACE),
        ("evmlib".to_string(), Level::TRACE),
        ("ant_logging".to_string(), Level::TRACE),
        ("ant_protocol".to_string(), Level::TRACE),
    ];
    let mut log_builder = LogBuilder::new(logging_targets);
    // log_builder.output_dest(opt.log_output_dest.clone());
    // log_builder.format(opt.log_format.unwrap_or(LogFormat::Default));
    let guards = log_builder.initialize()?;
    Ok(guards)
}
