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

//! # DwebClient is a wrapper for the Autonomi::client::client
//!
//! Provides a simple way to connect and fund a client
//! for interaction with the Autonomi peer-to-peer storage
//! network.
//!
use bytes::Bytes;
use color_eyre::Result;

use autonomi::client::data::DataAddress;
use autonomi::client::{payment::PaymentOption, Client, GetError};
use autonomi::{InitialPeersConfig, TransactionConfig};
use autonomi::{Network, Wallet};

use crate::autonomi::access::keys::load_evm_wallet_from_env;
use crate::token::{Rate, ShowCost};

/// Control how dweb uses and reports on selected Autonomi APIs
///
/// This allows use of the API to be made more reliable by enabling
/// numbers of retries for some operations, how uploads are conducted
/// and so on.
#[derive(Clone)]
pub struct ApiControl {
    /// Control number of tries on selected Autonomi calls (0 for unlimited)
    pub tries: u32,
    /// Do upload of directories one file at a time. Without this a retry will start from scratch.
    pub upload_file_by_file: bool,
    /// Control dweb APIs use of pointers.
    ///
    /// For selected APIs, if ignore_pointer is Some(true) the API will find
    /// the most recentry entry (head) of a graph by following the graph from the
    /// Pointer target to the end. When it is Some(false) the pointer is assumed
    /// to be up-to-date and point to the most recent graph entry.
    ///
    /// When None, behaviour depends on the API, but most will trust that
    /// the pointer is up-to-date and points to the most recent entry of
    /// of a graph.
    ///
    /// Can be used to investigate behaviour such as Pointers not updating on the public network.
    pub ignore_pointers: Option<bool>,
    /// Show the cost of dweb API calls after each call in tokens, gas, both or none
    pub show_dweb_costs: ShowCost,
    /// Optional maximum fee in Gwei (units of 0.000000001 ETH), to pay for a transaction on the Arbitrum network.
    pub max_fee_per_gas: Option<u128>,
}

impl Default for ApiControl {
    /// Note: some defaults are likely overriden by command line defaults passed when creating an DwebClient.
    fn default() -> Self {
        ApiControl {
            tries: 1,
            upload_file_by_file: false,
            ignore_pointers: None,
            show_dweb_costs: ShowCost::Both,
            max_fee_per_gas: None,
        }
    }
}

/// A wrapper for autonomi::Client which simplifies use of dweb APIs
/// TODO support separate data creation/owner and wallet keys
#[derive(Clone)]
pub struct DwebClient {
    pub client: Client,
    pub network: Network,
    pub wallet: Wallet, // Must be loaded and funded for writing to the network

    pub api_control: ApiControl,

    pub ant_rate: Option<Rate>,
    pub eth_rate: Option<Rate>,
}

impl DwebClient {
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
    pub async fn initialise_and_connect(
        peers: InitialPeersConfig,
        api_control: ApiControl,
    ) -> Result<DwebClient> {
        println!("Dweb Autonomi client initialising...");
        let client = crate::autonomi::actions::connect_to_network(peers).await?;

        let mut wallet = match load_evm_wallet_from_env(&client.evm_network()) {
            Ok(wallet) => wallet,
            Err(_e) => {
                let client = client.clone();
                println!("Failed to load wallet for payments - client will only have read accesss to Autonomi");
                Wallet::new_with_random_wallet(client.evm_network().clone())
            }
        };

        if let Some(max_fee_per_gas) = api_control.max_fee_per_gas {
            wallet.set_transaction_config(TransactionConfig::new(max_fee_per_gas));
            println!("Max fee per gas set to: {}", max_fee_per_gas);
        }
        let client = client.clone();
        let ant_rate = Rate::from_environment("ANT".to_string());
        let eth_rate = Rate::from_environment("ETH".to_string());
        Ok(DwebClient {
            client: client.clone(),
            network: client.evm_network().clone(),
            wallet,
            api_control,
            ant_rate,
            eth_rate,
        })
    }

    pub fn payment_option(&self) -> PaymentOption {
        PaymentOption::from(&self.wallet)
    }

    pub async fn data_get_public(&self, address: DataAddress) -> Result<Bytes, GetError> {
        self.client.data_get_public(&address).await
    }
}
