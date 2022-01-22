//! Swap tokens with the Raydium pool instruction
/*
use {
    solana_farm_sdk::{
        instruction::raydium::RaydiumSwap,
        program::{account, protocol::raydium},
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke,
        program_error::ProgramError,
    },
};
*/
use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundCustodyType},
        program::{account, pda, protocol::raydium},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program_error::ProgramError,
    },
};

pub fn raydium_swap(
    fund: &Fund,
    accounts: &[AccountInfo],
    token_a_amount_in: u64,
    token_b_amount_in: u64,
    min_token_amount_out: u64,
) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [admin_account, _fund_metadata, fund_info_account, fund_authority, fund_token_a_account, fund_token_b_account, pool_program_id, pool_coin_token_account, pool_pc_token_account, spl_token_program, amm_id, amm_authority, amm_open_orders, amm_target, serum_market, serum_program_id, serum_bids, serum_asks, serum_event_queue, serum_coin_vault_account, serum_pc_vault_account, serum_vault_signer] =
        accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        /*let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::InvalidArgument);
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if !raydium::check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        common::check_custody_account(
            fund,
            &token_a_metadata,
            fund_token_a_account,
            token_a_custody_metadata,
            FundCustodyType::Trading,
        )?;
        common::check_custody_account(
            fund,
            &token_b_metadata,
            fund_token_b_account,
            token_b_custody_metadata,
            FundCustodyType::Trading,
        )?;

        // get exact swap amounts
        let (amount_in, mut min_amount_out) = raydium::get_pool_swap_amounts(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
            token_a_amount_in,
            token_b_amount_in,
        )?;
        if min_token_amount_out > min_amount_out {
            min_amount_out = min_token_amount_out;
        }
        msg!(
            "Swap. amount_in: {}, min_amount_out {}",
            amount_in,
            min_amount_out
        );
        if amount_in == 0 || min_amount_out == 0 {
            msg!("Nothing to do: Not enough tokens to swap");
            return Ok(());
        }

        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        let initial_balance_in = if token_a_amount_in == 0 {
            account::get_token_balance(fund_token_b_account)?
        } else {
            account::get_token_balance(fund_token_a_account)?
        };
        let initial_balance_out = if token_a_amount_in == 0 {
            account::get_token_balance(fund_token_a_account)?
        } else {
            account::get_token_balance(fund_token_b_account)?
        };

        raydium::swap_with_seeds(
            &[
                fund_authority.clone(),
                if token_a_amount_in == 0 {
                    fund_token_b_account.clone()
                } else {
                    fund_token_a_account.clone()
                },
                if token_a_amount_in == 0 {
                    fund_token_a_account.clone()
                } else {
                    fund_token_b_account.clone()
                },
                pool_program_id.clone(),
                pool_coin_token_account.clone(),
                pool_pc_token_account.clone(),
                spl_token_program.clone(),
                amm_id.clone(),
                amm_authority.clone(),
                amm_open_orders.clone(),
                amm_target.clone(),
                serum_market.clone(),
                serum_program_id.clone(),
                serum_bids.clone(),
                serum_asks.clone(),
                serum_event_queue.clone(),
                serum_coin_vault_account.clone(),
                serum_pc_vault_account.clone(),
                serum_vault_signer.clone(),
            ],
            seeds,
            amount_in,
            min_amount_out,
        )?;

        let _ = account::check_tokens_spent(
            if token_a_amount_in == 0 {
                fund_token_b_account
            } else {
                fund_token_a_account
            },
            initial_balance_in,
            amount_in,
        )?;
        let tokens_received = account::check_tokens_received(
            if token_a_amount_in == 0 {
                fund_token_a_account
            } else {
                fund_token_b_account
            },
            initial_balance_out,
            min_amount_out,
        )?;

        msg!(
            "Done. tokens_received: {}, token_a_balance: {}, token_b_balance: {}",
            tokens_received,
            account::get_token_balance(fund_token_a_account)?,
            account::get_token_balance(fund_token_b_account)?
        );*/

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
