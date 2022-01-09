//! Fund Init instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        fund::{Fund, FundAssets},
        instruction::fund::FundInstruction,
        program::pda,
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn init(fund: &Fund, accounts: &[AccountInfo], step: u64) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [admin_account, _fund_metadata, fund_info_account, fund_authority, fund_program, _system_program, _spl_token_program, rent_program, fund_token_mint, fund_token_ref, vaults_assets_info, custodies_assets_info, liquidation_state] =
        accounts
    {
        // validate accounts
        if fund_authority.key != &fund.fund_authority
            || fund_token_ref.key != &fund.fund_token_ref
            || fund_program.key != &fund.fund_program_id
        {
            msg!("Error: Invalid Fund accounts");
            return Err(ProgramError::InvalidArgument);
        }

        // init fund authority account
        msg!("Init fund authority");
        pda::init_system_account(
            admin_account,
            fund_authority,
            &fund.fund_program_id,
            &fund.fund_program_id,
            &[b"fund_authority", fund.name.as_bytes()],
            0,
        )?;

        // init fund info account
        msg!("Init fund info");
        pda::init_system_account(
            admin_account,
            fund_info_account,
            &fund.fund_program_id,
            &fund.fund_program_id,
            &[b"info_account", fund.name.as_bytes()],
            FundInfo::LEN,
        )?;
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info.init(&fund.name)?;

        // init fund token mint
        msg!("Init fund token mint");
        let fund_token = Token::unpack(&fund_token_ref.try_borrow_data()?)?;
        if fund_token_mint.key != &fund_token.mint {
            msg!("Error: Invalid Fund token mint");
            return Err(ProgramError::InvalidArgument);
        }
        pda::init_mint(
            admin_account,
            fund_token_mint,
            fund_authority,
            rent_program,
            &fund.fund_program_id,
            &[b"fund_token_mint", fund.name.as_bytes()],
            fund_token.decimals,
        )?;

        // init vaults assets info
        msg!("Init vaults assets info");
        pda::init_system_account(
            admin_account,
            vaults_assets_info,
            &fund.fund_program_id,
            &fund.fund_program_id,
            &[b"vaults_assets_info", fund.name.as_bytes()],
            FundAssets::LEN,
        )?;

        // init custodies assets info
        msg!("Init custodies assets info");
        pda::init_system_account(
            admin_account,
            custodies_assets_info,
            &fund.fund_program_id,
            &fund.fund_program_id,
            &[b"custodies_assets_info", fund.name.as_bytes()],
            FundAssets::LEN,
        )?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
