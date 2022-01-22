//! Approve withdrawal from the Fund instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundUserInfo},
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
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
};

pub fn approve_withdrawal(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [admin_account, _fund_metadata, fund_info_account, fund_authority, _spl_token_program, fund_token_mint, user_account, user_info_account, user_withdrawal_token_account, user_fund_token_account, custody_account, custody_fees_account, custody_metadata, custody_token_metadata, pyth_price_info] =
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
        common::check_wd_custody_accounts(
            fund,
            &custody_token,
            user_withdrawal_token_account,
            custody_account,
            custody_fees_account,
            custody_metadata,
            pyth_price_info,
        )?;

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
        if user_info.withdrawal_request.amount == 0 {
            msg!("Error: No pending withdrawals found");
            return Err(ProgramError::InvalidArgument);
        }

        msg!("Compute withdrawal amount and fees");
        let amount = if amount == 0 {
            user_info.withdrawal_request.amount
        } else {
            std::cmp::min(amount, user_info.withdrawal_request.amount)
        };
        // 0 <= fund_fee <= 1
        let fund_fee = fund_info.get_withdrawal_fee()?;
        let withdrawal_fee = math::checked_as_u64(fund_fee * (amount as f64))?;
        let withdrawal_amount = amount - withdrawal_fee;
        if withdrawal_amount == 0 {
            msg!("Error: Insufficient user funds");
            return Err(ProgramError::InsufficientFunds);
        }
        let amount_with_fee = withdrawal_amount + withdrawal_fee;

        // check if last assets update was not too long ago,
        // stale value may lead to incorrect amount of tokens received
        common::check_assets_update_time(
            fund_info.get_assets_update_time()?,
            fund_info.get_assets_max_update_age_sec()?,
        )?;

        // compute nominal value of withdrawn tokens and check against the limit
        msg!("Compute assets value");
        let ft_supply_amount = account::get_token_supply(fund_token_mint)?;
        if amount_with_fee > ft_supply_amount {
            msg!("Error: Insufficient Fund supply amount");
            return Err(ProgramError::InsufficientFunds);
        }
        // ft_supply_amount > 0
        let withdrawal_value_usd =
            fund_info.get_current_assets_usd()? * amount_with_fee as f64 / ft_supply_amount as f64;

        msg!("Withdraw tokens from custody. withdrawal_amount: {}, withdrawal_fee: {}, withdrawal_value_usd: {}",
                withdrawal_amount, withdrawal_fee, withdrawal_value_usd);

        // compute tokens to transfer
        let tokens_to_remove = common::get_asset_value_tokens(
            withdrawal_value_usd,
            custody_token.decimals,
            fund_info.get_assets_max_price_error()?,
            fund_info.get_assets_max_price_age_sec()?,
            pyth_price_info,
        )?;
        let fee_tokens = math::checked_as_u64(fund_fee * (tokens_to_remove as f64))?;
        let tokens_to_tranfer = tokens_to_remove - fee_tokens;
        if tokens_to_tranfer == 0 {
            msg!("Error: Withdrawal amount is too small");
            return Err(ProgramError::InsufficientFunds);
        }
        if tokens_to_remove > account::get_token_balance(custody_account)? {
            msg!("Error: Withdrawal for this amount couldn't be completed at this time. Contact Fund administrator.");
            return Err(ProgramError::InsufficientFunds);
        }

        // transfer tokens from custody to the user
        msg!(
            "Transfer tokens to user wallet. tokens_to_tranfer: {}, fee_tokens: {}",
            tokens_to_tranfer,
            fee_tokens,
        );
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::transfer_tokens_with_seeds(
            custody_account,
            user_withdrawal_token_account,
            fund_authority,
            seeds,
            tokens_to_tranfer,
        )?;
        if fee_tokens > 0 {
            pda::transfer_tokens_with_seeds(
                custody_account,
                custody_fees_account,
                fund_authority,
                seeds,
                fee_tokens,
            )?;
        }

        // burn Fund tokens from user
        msg!(
            "Burn Fund tokens from the user. amount_with_fee {}",
            amount_with_fee
        );
        common::check_fund_token_mint(fund, fund_token_mint)?;
        pda::burn_tokens_with_seeds(
            user_fund_token_account,
            fund_token_mint,
            fund_authority,
            seeds,
            amount_with_fee,
        )?;

        // update stats
        msg!("Update Fund stats");
        let current_assets_usd = fund_info.get_current_assets_usd()?;
        let new_assets = if current_assets_usd > withdrawal_value_usd {
            current_assets_usd - withdrawal_value_usd
        } else {
            0.0
        };
        fund_info
            .set_amount_removed_usd(fund_info.get_amount_removed_usd()? + withdrawal_value_usd)?;
        fund_info.set_current_assets_usd(new_assets)?;
        fund_info.update_admin_action_time()?;

        // update user stats
        msg!("Update user stats");
        user_info.last_withdrawal.time = user_info.withdrawal_request.time;
        user_info.last_withdrawal.amount = withdrawal_amount;
        user_info.withdrawal_request.time = 0;
        user_info.withdrawal_request.amount = 0;
        user_info.deny_reason = ArrayString64::default();
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
