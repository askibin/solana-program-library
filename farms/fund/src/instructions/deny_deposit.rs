//! Deny deposit to the Fund instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundUserInfo},
        math,
        program::{account, clock, pda},
        string::{str_to_as64, ArrayString64},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        hash::Hasher,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
};

pub fn deny_deposit(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [admin_account, _fund_metadata, fund_info_account, fund_authority, _spl_token_program, user_account, user_info_account, user_deposit_token_account] =
        accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if user_fund_token_account.data_is_empty()
            || &account::get_token_account_owner(user_fund_token_account)? != user_account.key
        {
            msg!("Error: Invalid fund token account owner");
            return Err(ProgramError::IllegalOwner);
        }

        let mut user_info =
            if let Ok(user_info) = FundUserInfo::unpack(&user_info_account.try_borrow_data()?) {
                user_info
            } else {
                msg!("Failed to load user info account");
                return Err(ProgramError::InvalidAccountData);
            };
        common::check_user_info_account(
            fund,
            &custody_token,
            &user_info,
            user_account,
            user_info_account,
        )?;

        // check if there are any pending requests
        if user_info.deposit_request.amount == 0 {
            msg!("Error: No pending deposits found");
            return Err(ProgramError::InvalidArgument);
        }

        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        // update stats
        msg!("Update Fund stats");
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info
            .set_amount_invested_usd(fund_info.get_amount_invested_usd()? + deposit_value_usd)?;
        fund_info.set_current_assets_usd(current_assets_usd + deposit_value_usd)?;
        fund_info.update_admin_action_time()?;

        // update user stats
        msg!("Update user stats");
        user_info.last_deposit.time = user_info.deposit_request.time;
        user_info.last_deposit.amount = user_info.deposit_request.amount;
        user_info.deposit_request.time = 0;
        user_info.deposit_request.amount = 0;
        user_info.deny_reason = str_to_as64("")?;
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
