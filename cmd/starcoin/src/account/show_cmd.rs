// Copyright (c) The Starcoin Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::cli_state::CliState;
use crate::view::{AccountWithStateView, AddressOrReceipt};
use crate::StarcoinOpt;
use anyhow::{format_err, Result};
use scmd::{CommandAction, ExecContext};
use starcoin_crypto::{HashValue, ValidCryptoMaterialStringExt};
use starcoin_rpc_client::RemoteStateReader;
use starcoin_state_api::AccountStateReader;
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Default)]
/// Show a account info, only the accounts managed by the current node are supported
#[structopt(name = "show")]
pub struct ShowOpt {
    #[structopt(name = "address_or_receipt")]
    /// The account's address or receipt to show, if absent, show the default account.
    address_or_receipt: Option<AddressOrReceipt>,

    #[structopt(name = "block_id", short = "b")]
    block_id: Option<HashValue>,
}

pub struct ShowCommand;

impl CommandAction for ShowCommand {
    type State = CliState;
    type GlobalOpt = StarcoinOpt;
    type Opt = ShowOpt;
    type ReturnItem = AccountWithStateView;

    fn run(
        &self,
        ctx: &ExecContext<Self::State, Self::GlobalOpt, Self::Opt>,
    ) -> Result<Self::ReturnItem> {
        let client = ctx.state().client();
        let opt = ctx.opt();
        let account_address = if let Some(address_or_receipt) = opt.address_or_receipt {
            address_or_receipt.address()
        } else {
            let default_account = client
                .account_default()?
                .ok_or_else(|| format_err!("Default account should exist."))?;
            default_account.address
        };
        let account = client
            .account_get(account_address)?
            .ok_or_else(|| format_err!("Account with address {} not exist.", account_address))?;

        let chain_state_reader = if let Some(block_id) = opt.block_id {
            let block = client
                .chain_get_block_by_hash(block_id)?
                .ok_or_else(|| format_err!("block {} not found", block_id))?;
            RemoteStateReader::new_with_root(client, block.header.state_root)
        } else {
            RemoteStateReader::new(client)?
        };
        let account_state_reader = AccountStateReader::new(&chain_state_reader);
        let sequence_number = account_state_reader
            .get_account_resource(account.address())?
            .map(|res| res.sequence_number());

        let accepted_tokens = client.account_accepted_tokens(account_address)?;
        let mut balances = HashMap::with_capacity(accepted_tokens.len());
        for token in accepted_tokens {
            let token_name = token.name.clone();
            let balance =
                account_state_reader.get_balance_by_token_code(account.address(), token)?;
            if let Some(b) = balance {
                balances.insert(token_name, b);
            }
        }
        let auth_key = account.public_key.authentication_key();
        Ok(AccountWithStateView {
            auth_key: auth_key.to_encoded_string()?,
            account,
            sequence_number,
            balances,
        })
    }
}
