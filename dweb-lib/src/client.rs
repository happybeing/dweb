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
use color_eyre::Result;

use crate::token::{Rate, ShowCost};
use autonomi::client::{payment::PaymentOption, Client};
use autonomi::TransactionConfig;
use autonomi::{Network, Wallet};

use crate::autonomi::args::max_fee_per_gas::{
    get_max_fee_per_gas_from_opt_param, MaxFeePerGasParam,
};

/// Control how dweb uses and reports on selected Autonomi APIs
///
/// This allows use of the API to be made more reliable by enabling
/// numbers of retries for some operations, how uploads are conducted
/// and so on.
#[derive(Clone)]
pub struct ApiControl {
    /// Control number of tries on selected Autonomi calls (0 for unlimited)
    pub tries: u32,
    /// Use PublicArchive instead of PrivateArchive when storing directories
    pub use_public_archive: bool,
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
    pub ignore_pointers: bool,
    /// Show the cost of dweb API calls after each call in tokens, gas, both or none
    pub show_dweb_costs: ShowCost,
    /// Optional control maximum fee paid for a transaction on the Arbitrum network.
    pub max_fee_per_gas: Option<MaxFeePerGasParam>,
}

impl Default for ApiControl {
    /// Note: some defaults are likely overriden by command line defaults passed when creating an DwebClient.
    fn default() -> Self {
        ApiControl {
            tries: 1,
            use_public_archive: false,
            upload_file_by_file: false,
            ignore_pointers: false,
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
    pub is_local: bool,
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
        local_network: bool,
        alpha_network: bool,
        api_control: ApiControl,
    ) -> Result<DwebClient> {
        println!("Dweb Autonomi client initialising...");

        let client = if local_network {
            Client::init_local().await?
        } else if alpha_network {
            Client::init_alpha().await?
        } else {
            Client::init().await?
        };

        let mut wallet = match crate::autonomi::wallet::load_wallet(&client.evm_network()) {
            Ok(wallet) => wallet,
            Err(_e) => {
                let client = client.clone();
                println!("Failed to load wallet for payments - client will only have read accesss to Autonomi");
                Wallet::new_with_random_wallet(client.evm_network().clone())
            }
        };

        let max_fee_per_gas =
            get_max_fee_per_gas_from_opt_param(api_control.max_fee_per_gas, client.evm_network())?;
        wallet.set_transaction_config(TransactionConfig {
            max_fee_per_gas: max_fee_per_gas.clone(),
        });

        // println!
        println!("DEBUG loaded wallet: {}", wallet.address());
        println!(
            "DEBUG     tokens: {}",
            wallet.balance_of_tokens().await.unwrap()
        );
        println!(
            "DEBUG     gas   : {}",
            wallet.balance_of_gas_tokens().await.unwrap()
        );
        println!("Max fee per gas set to: {:?}", max_fee_per_gas);

        let client = client.clone();
        let ant_rate = Rate::from_environment("ANT".to_string());
        let eth_rate = Rate::from_environment("ETH".to_string());
        Ok(DwebClient {
            client: client.clone(),
            network: client.evm_network().clone(),
            is_local: local_network,
            wallet,
            api_control: api_control.clone(),
            ant_rate,
            eth_rate,
        })
    }

    pub fn payment_option(&self) -> PaymentOption {
        PaymentOption::from(&self.wallet)
    }
}
