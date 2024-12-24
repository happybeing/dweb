// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use autonomi::Client;
use autonomi::Multiaddr;
use color_eyre::eyre::bail;
use color_eyre::eyre::Result;
use indicatif::ProgressBar;
use std::time::Duration;

pub async fn connect_to_network(peers: Vec<Multiaddr>) -> Result<Client> {
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.enable_steady_tick(Duration::from_millis(120));
    progress_bar.set_message("Connecting to The Autonomi Network...");
    let new_style = progress_bar.style().tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈🔗");
    progress_bar.set_style(new_style);

    progress_bar.set_message("Connecting to The Autonomi Network...");

    match Client::init_with_peers(peers).await {
        Ok(client) => {
            info!("Connected to the Network");
            progress_bar.finish_with_message("Connected to the Network");
            Ok(client)
        }
        Err(e) => {
            error!("Failed to connect to the network: {e}");
            progress_bar.finish_with_message("Failed to connect to the network");
            bail!("Failed to connect to the network: {e}")
        }
    }
}

use color_eyre::eyre::Context;
// use color_eyre::Result;
use ant_bootstrap::{PeersArgs, ANT_PEERS_ENV};
use color_eyre::Section;

// TODO copied from dweb due to mismatch in PeersArgs
pub async fn get_peers(peers: PeersArgs) -> Result<Vec<Multiaddr>> {
    peers.get_addrs(None, Some(100)).await
        .wrap_err("Please provide valid Network peers to connect to")
        .with_suggestion(|| format!("make sure you've provided network peers using the --peers option or the {ANT_PEERS_ENV} env var"))
        .with_suggestion(|| "a peer address looks like this: /ip4/42.42.42.42/udp/4242/quic-v1/p2p/B64nodePeerIDvdjb3FAJF4ks3moreBase64CharsHere")
}
