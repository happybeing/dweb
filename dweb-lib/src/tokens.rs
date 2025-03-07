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

use color_eyre::{eyre::eyre, Result};

use autonomi::AttoTokens;
use autonomi::Wallet;
use evmlib::common::Amount;

use crate::client::AutonomiClient;

/// Control 'show cost' operations
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ShowCost {
    Token,
    Gas,
    Both,
    None,
}

pub struct Spends {
    pub token: Amount,
    pub gas: Amount,

    wallet: Wallet,

    show_cost: ShowCost,
    label: String,
}

/// Capture gas and token balances for monitoring and reporting spends
impl Spends {
    pub async fn new(client: &AutonomiClient, label: Option<&str>) -> Result<Spends> {
        let label = label.unwrap_or("Cost total: ").to_string();
        let wallet = client.wallet.clone();
        let show_cost = client.show_cost.clone();
        let token = wallet.balance_of_tokens().await?;
        let gas = wallet.balance_of_gas_tokens().await?;
        Ok(Spends {
            token: token,
            gas: gas,
            wallet,
            show_cost,
            label,
        })
    }

    pub async fn update(&mut self) -> Result<()> {
        self.token = self.wallet.balance_of_tokens().await?;
        self.gas = self.wallet.balance_of_gas_tokens().await?;
        Ok(())
    }

    /// Print the spend since last 'update' with optional label (which defaults to "Cost total: ")
    pub async fn show_spend(&self) -> Result<()> {
        let label = &self.label;
        let spent_gas = AttoTokens::from(self.spent_gas().await?);
        let spent_tokens = AttoTokens::from(self.spent_tokens().await?);
        match self.show_cost {
            ShowCost::Gas => {
                println!("{label}{spent_gas} Gas");
            }
            ShowCost::Token => {
                println!("{label}{spent_tokens} ANT");
            }
            ShowCost::Both => {
                println!("{label}{spent_gas} Gas");
                println!("{label}{spent_tokens} ANT");
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn spent_tokens(&self) -> Result<Amount> {
        let balance = self.wallet.balance_of_tokens().await?;
        match self.token.checked_sub(balance) {
            Some(spent) => Ok(spent),
            None => Err(eyre!("Error calculating spent tokens")),
        }
    }

    pub async fn spent_gas(&self) -> Result<Amount> {
        let balance = self.wallet.balance_of_gas_tokens().await?;
        match self.gas.checked_sub(balance) {
            Some(spent) => Ok(spent),
            None => {
                println!("Error calculating spent gas at balance.checked_sub(self.gas)");
                Err(eyre!("Error calculating spent gas"))
            }
        }
    }
}

/// Helper to simplify handling of Result<_>.
///
/// Return show_spend_return_value<T>() with T as the type of the return value you need
pub async fn show_spend_return_value<T>(spends: &Spends, value: T) -> T {
    let _ = spends.show_spend().await;
    value
}
