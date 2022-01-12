//! Fund AddCustody instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        fund::{Fund, FundAssets, FundCustody, FundCustodyType, DISCRIMINATOR_FUND_CUSTODY},
        instruction::fund::FundInstruction,
        program::{clock, pda},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn add_custody(
    fund: &Fund,
    accounts: &[AccountInfo],
    target_hash: u64,
    custody_id: u32,
    custody_type: FundCustodyType,
) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [admin_account, fund_metadata, fund_info_account, fund_authority, _system_program, _spl_token_program, rent_program, custodies_assets_info, custody_account, custody_fees_account, custody_metadata, custody_token_metadata, custody_token_mint, pyth_price_info] =
        accounts
    {
        // validate accounts
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::InvalidArgument);
        }

        if !custody_account.data_is_empty() || !custody_fees_account.data_is_empty() {
            msg!("Custody accounts must be uninitialized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // init custody metadata account
        msg!("Init custody metadata account");
        let custody_token =
            if let Ok(token) = Token::unpack(&custody_token_metadata.try_borrow_data()?) {
                token
            } else {
                msg!("Failed to load custody token metadata");
                return Err(ProgramError::InvalidAccountData);
            };
        let custody_seed_str: &[u8] = match custody_type {
            FundCustodyType::DepositWithdraw => b"fund_wd_custody_info",
            FundCustodyType::Trading => b"fund_trading_custody_info",
            _ => unreachable!(),
        };
        let custody_seeds = &[
            custody_seed_str,
            custody_token.name.as_bytes(),
            fund.name.as_bytes(),
        ];
        let bump = Pubkey::find_program_address(custody_seeds, &fund.fund_program_id).1;
        pda::init_system_account(
            admin_account,
            custody_metadata,
            &fund.fund_program_id,
            &fund.fund_program_id,
            custody_seeds,
            FundCustody::LEN,
        )?;

        let custody = FundCustody {
            discriminator: DISCRIMINATOR_FUND_CUSTODY,
            fund_ref: *fund_metadata.key,
            custody_id,
            custody_type,
            token_ref: *custody_token_metadata.key,
            address: *custody_account.key,
            fees_address: *custody_fees_account.key,
            pyth_price_info: *pyth_price_info.key,
            liquidation_id: 0,
            liquidation_token_amount: 0,
            bump,
        };
        custody.pack(*custody_metadata.try_borrow_mut_data()?)?;

        // init token accounts
        msg!("Init custody token account");
        let custody_seed_str: &[u8] = match custody_type {
            FundCustodyType::DepositWithdraw => b"fund_wd_custody_account",
            FundCustodyType::Trading => b"fund_trading_custody_account",
            _ => unreachable!(),
        };
        pda::init_token_account(
            admin_account,
            custody_account,
            custody_token_mint,
            fund_authority,
            rent_program,
            &fund.fund_program_id,
            &[
                custody_seed_str,
                custody_token.name.as_bytes(),
                fund.name.as_bytes(),
            ],
        )?;

        msg!("Init fee custody token account");
        let custody_seed_str: &[u8] = match custody_type {
            FundCustodyType::DepositWithdraw => b"fund_wd_custody_fees_account",
            FundCustodyType::Trading => b"fund_td_custody_fees_account",
            _ => unreachable!(),
        };
        pda::init_token_account(
            admin_account,
            custody_fees_account,
            custody_token_mint,
            fund_authority,
            rent_program,
            &fund.fund_program_id,
            &[
                custody_seed_str,
                custody_token.name.as_bytes(),
                fund.name.as_bytes(),
            ],
        )?;

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
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
