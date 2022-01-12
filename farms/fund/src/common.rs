//! Common functions

use {
    crate::fund_info::FundInfo,
    pyth_client::{CorpAction, PriceStatus, PriceType},
    solana_farm_sdk::{
        fund::{Fund, FundCustody, FundCustodyType, FundUserInfo},
        id::zero,
        math,
        program::{account, clock},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, clock::UnixTimestamp, entrypoint::ProgramResult, msg,
        program_error::ProgramError, pubkey::Pubkey,
    },
    std::cmp,
};

pub fn check_wd_custody_accounts<'a, 'b>(
    fund: &Fund,
    custody_token: &Token,
    user_deposit_token_account: &'a AccountInfo<'b>,
    custody_account: &'a AccountInfo<'b>,
    custody_fees_account: &'a AccountInfo<'b>,
    custody_metadata: &'a AccountInfo<'b>,
    pyth_price_info: &'a AccountInfo<'b>,
) -> ProgramResult {
    //if custody_metadata.owner != &fund.fund_program_id
    //    || &account::get_token_account_owner(custody_account)? != &fund.fund_program_id
    //{
    //    msg!("Error: Invalid custody owner");
    //    return Err(ProgramError::IllegalOwner);
    //}
    let deposit_token_mint =
        if let Ok(mint) = account::get_token_account_mint(user_deposit_token_account) {
            mint
        } else {
            msg!("Error: Invalid user's deposit token account");
            return Err(ProgramError::InvalidAccountData);
        };
    let custody_account_mint = if let Ok(mint) = account::get_token_account_mint(custody_account) {
        mint
    } else {
        msg!("Error: Invalid custody token account");
        return Err(ProgramError::InvalidAccountData);
    };
    let custody_fees_account_mint =
        if let Ok(mint) = account::get_token_account_mint(custody_fees_account) {
            mint
        } else {
            msg!("Error: Invalid custody fees token account");
            return Err(ProgramError::InvalidAccountData);
        };
    if custody_token.mint != custody_account_mint
        || deposit_token_mint != custody_account_mint
        || deposit_token_mint != custody_fees_account_mint
    {
        msg!("Error: Custody mint mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    let custody = if let Ok(custody) = FundCustody::unpack(&custody_metadata.try_borrow_data()?) {
        custody
    } else {
        msg!("Failed to load custody metadata");
        return Err(ProgramError::InvalidAccountData);
    };
    let custody_metadata_derived = Pubkey::create_program_address(
        &[
            b"fund_wd_custody_info",
            custody_token.name.as_bytes(),
            fund.name.as_bytes(),
            &[custody.bump],
        ],
        &fund.fund_program_id,
    )?;
    if &custody_metadata_derived != custody_metadata.key
        || &custody.address != custody_account.key
        || &custody.fees_address != custody_fees_account.key
        || &custody.pyth_price_info != pyth_price_info.key
    {
        msg!("Error: Invalid custody accounts");
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}

pub fn check_custody_account<'a, 'b>(
    fund: &Fund,
    custody_token: &Token,
    custody_account: &'a AccountInfo<'b>,
    custody_metadata: &'a AccountInfo<'b>,
    custody_type: FundCustodyType,
) -> ProgramResult {
    //if custody_metadata.owner != &fund.fund_program_id
    //    || &account::get_token_account_owner(custody_account)? != &fund.fund_program_id
    //{
    //    msg!("Error: Invalid custody owner");
    //    return Err(ProgramError::IllegalOwner);
    //}
    let custody_account_mint = if let Ok(mint) = account::get_token_account_mint(custody_account) {
        mint
    } else {
        msg!("Error: Invalid custody token account");
        return Err(ProgramError::InvalidAccountData);
    };
    if custody_token.mint != custody_account_mint {
        msg!("Error: Custody mint mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    let custody = if let Ok(custody) = FundCustody::unpack(&custody_metadata.try_borrow_data()?) {
        custody
    } else {
        msg!("Failed to load custody metadata");
        return Err(ProgramError::InvalidAccountData);
    };
    let custody_seed_str: &[u8] = match custody_type {
        FundCustodyType::DepositWithdraw => b"fund_wd_custody_info",
        FundCustodyType::Trading => b"fund_trading_custody_info",
        _ => unreachable!(),
    };
    let custody_metadata_derived = Pubkey::create_program_address(
        &[
            custody_seed_str,
            custody_token.name.as_bytes(),
            fund.name.as_bytes(),
            &[custody.bump],
        ],
        &fund.fund_program_id,
    )?;
    if &custody_metadata_derived != custody_metadata.key || &custody.address != custody_account.key
    {
        msg!("Error: Invalid custody accounts");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub fn check_user_info_account<'a, 'b>(
    fund: &Fund,
    custody_token: &Token,
    user_info: &FundUserInfo,
    user_account: &'a AccountInfo<'b>,
    user_info_account: &'a AccountInfo<'b>,
) -> ProgramResult {
    //if user_info_account.owner != &fund.fund_program_id {
    //    msg!("Error: Invalid user info account owner");
    //    return Err(ProgramError::IllegalOwner);
    //}
    let user_info_derived = Pubkey::create_program_address(
        &[
            b"user_info_account",
            custody_token.name.as_bytes(),
            user_account.key.as_ref(),
            fund.name.as_bytes(),
            &[user_info.bump],
        ],
        &fund.fund_program_id,
    )?;
    if user_info_account.key != &user_info_derived {
        msg!("Error: Invalid user info address");
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}

pub fn check_fund_token_mint<'a, 'b>(
    fund: &Fund,
    fund_token_mint: &'a AccountInfo<'b>,
) -> ProgramResult {
    //if account::get_mint_authority(fund_token_mint)? != &fund.fund_program_id {
    //    msg!("Error: Invalid fund token mint authority");
    //    return Err(ProgramError::IllegalOwner);
    //}
    let fund_token_mint_derived = Pubkey::create_program_address(
        &[
            b"fund_token_mint",
            fund.name.as_bytes(),
            &[fund.fund_token_bump],
        ],
        &fund.fund_program_id,
    )?;
    if fund_token_mint.key != &fund_token_mint_derived {
        msg!("Error: Invalid fund token mint");
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}

pub fn check_assets_update_time(
    assets_update_time: UnixTimestamp,
    max_update_age_sec: u64,
) -> ProgramResult {
    let last_update_age_sec = math::checked_sub(clock::get_time()?, assets_update_time)?;
    if last_update_age_sec > max_update_age_sec as i64 {
        msg!("Error: Assets balance is stale. Contact Fund administrator.");
        Err(ProgramError::Custom(222))
    } else {
        Ok(())
    }
}

pub fn check_assets_limit_usd(
    fund_info: &FundInfo,
    deposit_value_usd: f64,
) -> Result<(), ProgramError> {
    let current_assets_usd = fund_info.get_current_assets_usd()?;
    let assets_limit = fund_info.get_assets_limit_usd()?;
    if assets_limit > 0.0 {
        if assets_limit < deposit_value_usd + current_assets_usd {
            let amount_left = if current_assets_usd < assets_limit {
                assets_limit - current_assets_usd
            } else {
                0.0
            };
            msg!(
                "Error: Fund assets limit reached ({}). Allowed max desposit USD: {}",
                assets_limit,
                amount_left
            );
            return Err(ProgramError::Custom(223));
        }
    }
    Ok(())
}

pub fn get_asset_value_usd<'a, 'b>(
    amount: u64,
    decimals: u8,
    max_price_error: f64,
    max_price_age_sec: u64,
    pyth_price_info: &'a AccountInfo<'b>,
) -> Result<f64, ProgramError> {
    if amount == 0 {
        return Ok(0.0);
    }
    if pyth_price_info.data_is_empty() {
        msg!("Error: Invalid Pyth oracle account");
        return Err(ProgramError::Custom(300));
    }

    let pyth_price_data = &pyth_price_info.try_borrow_data()?;
    let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

    if !matches!(pyth_price.agg.status, PriceStatus::Trading)
        || !matches!(pyth_price.ptype, PriceType::Price)
    {
        msg!("Error: Pyth oracle price has invalid state");
        return Err(ProgramError::Custom(301));
    }

    let last_update_age_sec = math::checked_mul(
        math::checked_sub(clock::get_slot()?, pyth_price.valid_slot)?,
        solana_program::clock::DEFAULT_MS_PER_SLOT,
    )? / 1000;
    if last_update_age_sec > max_price_age_sec {
        msg!("Error: Pyth oracle price is stale");
        return Err(ProgramError::Custom(302));
    }

    if pyth_price.agg.price <= 0
        || pyth_price.agg.conf as f64 / pyth_price.agg.price as f64 > max_price_error
    {
        msg!("Error: Pyth oracle price is out of bounds");
        return Err(ProgramError::Custom(303));
    }

    Ok(amount as f64 * pyth_price.agg.price as f64
        / f64::powi(10.0, decimals as i32 - pyth_price.expo))
}

pub fn get_fund_token_to_mint_amount(
    current_assets_usd: f64,
    deposit_amount: u64,
    deposit_value_usd: f64,
    ft_supply_amount: u64,
) -> Result<u64, ProgramError> {
    let ft_to_mint = if ft_supply_amount == 0 {
        deposit_amount
    } else if current_assets_usd <= 0.0001 {
        msg!("Error: Assets balance is stale. Contact Fund administrator.");
        return Err(ProgramError::Custom(222));
    } else {
        account::to_token_amount(
            deposit_value_usd as f64 / current_assets_usd as f64 * ft_supply_amount as f64,
            0,
        )?
    };
    Ok(ft_to_mint)
}
