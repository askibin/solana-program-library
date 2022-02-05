//! Fund RemoveCustody instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundAssets, FundCustodyType},
        program::{account, pda},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn remove_custody(
    fund: &Fund,
    accounts: &[AccountInfo],
    target_hash: u64,
    custody_type: FundCustodyType,
) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [admin_account, _fund_metadata, fund_info_account, fund_authority, _system_program, _spl_token_program, custodies_assets_info, custody_account, custody_fees_account, custody_metadata, custody_token_metadata] =
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
            custody_account,
            custody_metadata,
            custody_type,
        )?;

        // close accounts
        msg!("Close custody token accounts");
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::close_token_account_with_seeds(admin_account, custody_account, fund_authority, seeds)?;
        pda::close_token_account_with_seeds(
            admin_account,
            custody_fees_account,
            fund_authority,
            seeds,
        )?;

        msg!("Close custody metadata account");
        account::close_system_account(admin_account, custody_metadata, &fund.fund_program_id)?;

        // update assets tracking account
        msg!("Update Fund assets account");
        let custodies_assets_info_derived = Pubkey::find_program_address(
            &[b"custodies_assets_info", fund.name.as_bytes()],
            &fund.fund_program_id,
        )
        .0;
        if &custodies_assets_info_derived != custodies_assets_info.key {
            msg!("Error: Invalid custody accounts");
            return Err(ProgramError::InvalidArgument);
        }

        let mut fund_assets = if let Ok(fund_assets) =
            FundAssets::unpack(&custodies_assets_info.try_borrow_data()?)
        {
            fund_assets
        } else {
            msg!("Failed to load Fund assets account");
            return Err(ProgramError::InvalidAccountData);
        };
        fund_assets.current_hash = 0;
        fund_assets.target_hash = target_hash;
        fund_assets.pack(*custodies_assets_info.try_borrow_mut_data()?)?;

        // update fund stats
        msg!("Update Fund stats");
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
