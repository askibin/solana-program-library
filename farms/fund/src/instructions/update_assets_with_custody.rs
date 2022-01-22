//! Update Fund assets with custody balance instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundCustody, FundUserInfo},
        math,
        program::{account, clock, pda},
        string::ArrayString64,
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        hash::Hasher,
        instruction::{AccountMeta, Instruction},
        log::sol_log_compute_units,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
};

pub fn update_assets_with_custody(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [user_account, _fund_metadata, fund_info_account, custodies_assets_info, custody_account, custody_metadata, custody_token_metadata] =
        accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::InvalidArgument);
        }

        let custody_token =
            if let Ok(token) = Token::unpack(&custody_token_metadata.try_borrow_data()?) {
                token
            } else {
                msg!("Failed to load custody token metadata");
                return Err(ProgramError::InvalidAccountData);
            };
        let custody = if let Ok(custody) = FundCustody::unpack(&custody_metadata.try_borrow_data()?)
        {
            custody
        } else {
            msg!("Failed to load custody metadata");
            return Err(ProgramError::InvalidAccountData);
        };
        common::check_custody_account(
            fund,
            &custody_token,
            custody_account,
            custody_metadata,
            custody.custody_type,
        )?;

        fund_info.set_assets_update_time(clock::get_time()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
