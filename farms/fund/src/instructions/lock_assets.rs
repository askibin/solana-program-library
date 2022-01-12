//! Accept and move funds inside the Fund instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundCustodyType},
        program::{account, pda},
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

pub fn lock_assets(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [_admin_account, _fund_metadata, fund_info_account, fund_authority, _spl_token_program, wd_custody_account, wd_custody_metadata, trading_custody_account, trading_custody_metadata, custody_token_metadata] =
        accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::InvalidArgument);
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund accounts");
            return Err(ProgramError::InvalidArgument);
        }

        let custody_token =
            if let Ok(token) = Token::unpack(&custody_token_metadata.try_borrow_data()?) {
                token
            } else {
                msg!("Failed to load custody token metadata");
                return Err(ProgramError::InvalidAccountData);
            };
        common::check_custody_account(
            fund,
            &custody_token,
            wd_custody_account,
            wd_custody_metadata,
            FundCustodyType::DepositWithdraw,
        )?;
        common::check_custody_account(
            fund,
            &custody_token,
            trading_custody_account,
            trading_custody_metadata,
            FundCustodyType::Trading,
        )?;

        // check if there are funds in w/d custody
        let wd_custody_balance = account::get_token_balance(wd_custody_account)?;
        let amount = if amount > 0 {
            amount
        } else {
            wd_custody_balance
        };
        if amount == 0 || amount < wd_custody_balance {
            msg!("Error: Not enough funds in w/d custody");
            return Err(ProgramError::InvalidArgument);
        }

        // trandsfer tokens from w/d to trading custody
        msg!("Transfer funds to trading custody");
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::transfer_tokens_with_seeds(
            wd_custody_account,
            trading_custody_account,
            fund_authority,
            seeds,
            amount,
        )?;

        // update stats
        msg!("Update Fund stats");
        fund_info.update_admin_action_time()?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
