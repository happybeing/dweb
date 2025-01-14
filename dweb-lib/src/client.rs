/*
Copyright (c) 2024-2025 Mark Hughes

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

//! # AutonomiClient is a wrapper for the Autonomi::client::client
//!
//! Provides a simple way to connect and fund a client
//! for interaction with the Autonomi peer-to-peer storage
//! network.
//!
use bytes::Bytes;
use color_eyre::{eyre::eyre, Result};
use ring::agreement::PublicKey;
use xor_name::XorName as FileAddress;

use ant_bootstrap::PeersArgs;
use autonomi::client::data::GetError;
use autonomi::client::registers::{Register, RegisterSecretKey};
use autonomi::client::Client;

use autonomi::{get_evm_network_from_env, ClientConfig, Network, Wallet};

use crate::autonomi::access::keys::get_register_signing_key;
use crate::autonomi::wallet::load_wallet;

#[derive(Clone)]
pub struct AutonomiClient {
    pub client: Client,
    pub network: Network,
    pub wallet: Wallet,
    pub register_secret: Option<RegisterSecretKey>,
}

impl AutonomiClient {
    /// Create and initialse a client ready to access Autonomi
    ///
    /// Sets the default Ethereum Virtual Machine (EVM), obtains peers,
    /// attempts to connect to the network and creates a wallet for use
    /// by the client.
    ///
    /// If a wallet is present and funded on your system it will be used
    /// by the client to pay for storing data. You can override the wallet
    /// to be used by setting the SECRET_KEY environment variable to the
    /// private key of the wallet you wish to use which is handy for testing.
    ///
    /// The EMV network can be overridden by setting the EVM_NETWORK environment
    /// variable. For example, setting this to 'arbitrum-sepolia' selects the
    /// Artbitrum test network.
    pub async fn initialise_and_connect(peers_args: Option<PeersArgs>) -> Result<AutonomiClient> {
        let network = match get_evm_network_from_env() {
            Ok(network) => network,
            Err(_e) => Network::default(),
        };
        println!("DEBUG: selected network {network:?}");

        let peers_args = if peers_args.is_some() {
            peers_args.unwrap()
        } else {
            PeersArgs::default()
        };

        let mut client_config = ClientConfig::default();

        let client = match crate::autonomi::access::network::get_peers(peers_args).await {
            Ok(peers) => {
                client_config.peers = Some(peers);

                match Client::init_with_config(client_config).await {
                    Ok(client) => {
                        println!("DEBUG: Connected to the Network");
                        client
                    }
                    Err(e) => return Err(eyre!("Failed to connect to the network: {e}")),
                }
            }
            Err(e) => return Err(eyre!("Failed to get peers: {e}")),
        };

        // TODO: may become redundant (PR #2613: https://github.com/maidsafe/autonomi/pull/2613?notification_referrer_id=NT_kwDOACFS17MxNDE5MjMxNDExMzoyMTgzODk1&notifications_query=is%3Aunread)
        let wallet = match load_wallet() {
            Ok(wallet) => wallet,
            Err(_e) => {
                println!("Failed to load wallet for payments - client will only have read accesss to Autonomi");
                Wallet::new_with_random_wallet(network.clone())
            }
        };

        let register_secret = match get_register_signing_key() {
            Ok(register_secret) => Some(register_secret),
            Err(e) => {
                println!("Register signing key not found.\nThe signing key is Not needed for browsing, but use 'ant register generate-key' if you wish to publish.");
                None
            }
        };

        Ok(AutonomiClient {
            client,
            network,
            wallet,
            register_secret,
        })
    }

    pub async fn data_get_public(&self, address: FileAddress) -> Result<Bytes, GetError> {
        self.client.data_get_public(address).await
    }
}
