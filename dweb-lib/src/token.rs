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
use evmlib::common::{Amount, U256};

use crate::client::DwebClient;

/// Control 'show cost' operations
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ShowCost {
    Token,
    Gas,
    Both,
    None,
}

#[derive(Clone)]
pub struct Spends {
    client: DwebClient,
    pub token: Amount,
    pub gas: Amount,

    show_cost: ShowCost,
    label: String,
}

/// Capture gas and token balances for monitoring and reporting spends
impl Spends {
    pub async fn new(client: &DwebClient, label: Option<&str>) -> Result<Spends> {
        let label = label.unwrap_or("Cost total: ").to_string();
        let client = client.clone();
        let show_cost = client.api_control.show_dweb_costs.clone();
        let token = client.wallet.balance_of_tokens().await?;
        let gas = client.wallet.balance_of_gas_tokens().await?;
        Ok(Spends {
            token: token,
            gas: gas,
            client,
            show_cost,
            label,
        })
    }

    pub async fn update(&mut self) -> Result<()> {
        self.token = self.client.wallet.balance_of_tokens().await?;
        self.gas = self.client.wallet.balance_of_gas_tokens().await?;
        Ok(())
    }

    /// Print the spend since last 'update' with optional label (which defaults to "Cost total: ")
    pub async fn show_spend(&self) -> Result<()> {
        let label = &self.label;
        let spent_gas = AttoTokens::from(self.spent_gas().await?);
        let spent_gas_string = format_tokens(spent_gas.as_atto());

        let spent_tokens = AttoTokens::from(self.spent_tokens().await?);
        let spent_tokens_string = format_tokens(spent_tokens.as_atto());

        let spent_gas = if let Some(eth_rate) = &self.client.eth_rate {
            format!(
                "{label}{} ({spent_gas_string} Gas)",
                eth_rate.to_currency(&spent_gas)
            )
        } else {
            format!("{label}{spent_gas_string} Gas")
        };
        let spent_tokens = if let Some(ant_rate) = &self.client.ant_rate {
            format!(
                "{label}{} ({spent_tokens_string} ANT)",
                ant_rate.to_currency(&spent_tokens)
            )
        } else {
            format!("{label}{spent_tokens_string} ANT")
        };

        match self.show_cost {
            ShowCost::Gas => {
                println!("{spent_gas}");
            }
            ShowCost::Token => {
                println!("{spent_tokens}");
            }
            ShowCost::Both => {
                println!("{spent_gas}");
                println!("{spent_tokens}");
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn spent_tokens(&self) -> Result<Amount> {
        let balance = self.client.wallet.balance_of_tokens().await?;
        match self.token.checked_sub(balance) {
            Some(spent) => Ok(spent),
            None => Err(eyre!("Error calculating spent tokens")),
        }
    }

    pub async fn spent_gas(&self) -> Result<Amount> {
        let balance = self.client.wallet.balance_of_gas_tokens().await?;
        match self.gas.checked_sub(balance) {
            Some(spent) => Ok(spent),
            None => {
                println!("Error calculating spent gas at balance.checked_sub(self.gas)");
                Err(eyre!("Error calculating spent gas"))
            }
        }
    }
}

const UNITS_PER_TOKEN_U64: u64 = 1_000_000_000_000_000_000;
const UNITS_PER_TOKEN_F32: f32 = 1_000_000_000_000_000_000.0;

/// Return a string representation with 18 decimal places
pub fn format_tokens(amount: Amount) -> String {
    let unit = amount / Amount::from(UNITS_PER_TOKEN_U64);
    let remainder = amount % Amount::from(UNITS_PER_TOKEN_U64);
    format!("{unit}.{remainder:018}").to_string()
}

/// Helper to simplify handling of Result<_>.
///
/// Return show_spend_return_value<T>() with T as the type of the return value you need
pub async fn show_spend_return_value<T>(spends: &Spends, value: T) -> T {
    let _ = spends.show_spend().await;
    value
}

const RATE_VAR_PREFIX: &str = "DWEB_RATE_";

#[derive(Clone)]
pub struct Rate {
    pub ticker: String,   // ANT or ETH
    pub currency: String, // GBP, USD etc
    pub rate: f32,
    // pub date:   Option<time>,
}

impl Rate {
    pub fn from_environment(ticker: String) -> Option<Rate> {
        let env_var = Self::env_var_for(&ticker);
        let env_value = match std::env::var(&env_var) {
            Ok(value) => value,
            Err(_) => return None,
        };

        let mut iter = env_value.split(',');
        let rate = match iter.next().unwrap_or("0.0").parse::<f32>() {
            Ok(rate) => rate,
            Err(_) => return None,
        };

        let currency = iter.next().unwrap_or("ERROR").to_string();

        // TODO parse any date string into a date-time type so users can calculate the age of a rate
        let _date = iter.next().unwrap_or("ERROR");

        Some(Rate {
            ticker: ticker.clone(),
            currency,
            rate,
        })
    }

    pub fn env_var_for(ticker: &String) -> String {
        return format!("{RATE_VAR_PREFIX}{ticker}");
    }

    pub fn to_currency(&self, tokens: &AttoTokens) -> String {
        const MIN_FACTOR: f32 = 100_f32; // One power of ten per decimal place
        let factor = match self.rate {
            rate if rate < 0.001 => MIN_FACTOR * 1000_f32,
            rate if rate < 0.01 => MIN_FACTOR * 100_f32,
            rate if rate < 0.1 => MIN_FACTOR * 10_f32,
            rate if rate < 1. => MIN_FACTOR,
            rate if rate >= 1. => MIN_FACTOR,
            _ => 1.0, // NaN
        };

        // Scale up the rate by factor -> f32 an
        let scaled_rate = U256::from(self.rate * factor);
        let scaled_value = scaled_rate * tokens.as_atto();
        let scaled_value = scaled_value.to::<u64>();
        let value = match format!("{scaled_value}").parse::<f32>() {
            Ok(scaled_value) => (scaled_value / factor) / UNITS_PER_TOKEN_F32,
            Err(_) => {
                return "[Invalid value]".to_string();
            }
        };

        format!("{}{value:.8}", self.currency).to_string()
    }
}
