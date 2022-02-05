//! Request deposit to the Fund instruction handler

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
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn request_deposit(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [user_account, _fund_metadata, fund_info_account, fund_authority, _spl_token_program, fund_token_mint, user_info_account, user_deposit_token_account, user_fund_token_account, custody_account, custody_fees_account, custody_metadata, custody_token_metadata, pyth_price_info] =
        accounts
    {
        // return early if deposits are not allowed
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if !fund_info.is_deposit_allowed()? {
            msg!("Error: Deposits to this Fund are not allowed at this time");
            return Err(ProgramError::Custom(220));
        }
        // validate accounts
        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if user_fund_token_account.data_is_empty()
            || !account::check_token_account_owner(user_fund_token_account, user_account.key)?
        {
            msg!("Error: Invalid Fund token account owner");
            return Err(ProgramError::IllegalOwner);
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
            user_deposit_token_account,
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
        if user_info.withdrawal_request.amount != 0 {
            msg!("Error: Pending withdrawal must be canceled first");
            return Err(ProgramError::InvalidArgument);
        }
        if user_info.deposit_request.amount != 0 {
            msg!("Error: Pending deposit must be canceled first");
            return Err(ProgramError::InvalidArgument);
        }

        msg!("Compute deposit amount and fees");
        // if specified amount is zero compute it based on user's balance
        let user_token_balance = account::get_token_balance(user_deposit_token_account)?;
        let deposit_amount;
        let deposit_fee;
        // 0 <= fund_fee <= 1
        let fund_fee = fund_info.get_deposit_fee()?;
        if amount == 0 {
            if fund_fee == 0.0 {
                deposit_amount = user_token_balance;
                deposit_fee = 0;
            } else {
                // unchecked but safe math
                deposit_amount = (user_token_balance as f64 / (1.0 + fund_fee)) as u64;
                deposit_fee = user_token_balance - deposit_amount;
            }
        } else {
            deposit_fee = math::checked_as_u64(fund_fee * (amount as f64))?;
            deposit_amount = amount - deposit_fee;
        }
        if deposit_amount == 0 || deposit_amount + deposit_fee > user_token_balance {
            msg!("Error: Insufficient user funds");
            return Err(ProgramError::InsufficientFunds);
        }
        let amount_with_fee = deposit_amount + deposit_fee;

        // compute nominal value of deposited tokens and check against the limit
        msg!("Compute assets value");
        let deposit_value_usd = common::get_asset_value_usd(
            deposit_amount,
            custody_token.decimals,
            fund_info.get_assets_max_price_error()?,
            fund_info.get_assets_max_price_age_sec()?,
            pyth_price_info,
        )?;

        // check for deposit limit
        let deposit_limit = fund_info.get_deposit_limit_usd()?;
        if deposit_limit > 0.0 && deposit_limit < deposit_value_usd {
            msg!(
                "Error: Deposit amount {} is over the limit {}",
                deposit_value_usd,
                deposit_limit
            );
            return Err(ProgramError::Custom(221));
        }

        if !fund_info.is_deposit_approval_required()? {
            // if no approval required try to perform deposit instantly
            msg!(
                "Deposit tokens into custody. deposit_amount: {}, deposit_fee: {}, deposit_value_usd: {}",
                deposit_amount,
                deposit_fee,
                deposit_value_usd
            );

            // check for total asset amount limit
            common::check_assets_limit_usd(&fund_info, deposit_value_usd)?;

            // check if last assets update was not too long ago,
            // stale value may lead to incorrect amount of fund tokens minted
            common::check_assets_update_time(
                fund_info.get_assets_update_time()?,
                fund_info.get_assets_max_update_age_sec()?,
            )?;
            account::transfer_tokens(
                user_deposit_token_account,
                custody_account,
                user_account,
                deposit_amount,
            )?;
            if deposit_fee > 0 {
                account::transfer_tokens(
                    user_deposit_token_account,
                    custody_fees_account,
                    user_account,
                    deposit_fee,
                )?;
            }

            // mint Fund tokens to user
            let current_assets_usd = fund_info.get_current_assets_usd()?;
            let ft_supply_amount = account::get_token_supply(fund_token_mint)?;
            let ft_to_mint = common::get_fund_token_to_mint_amount(
                current_assets_usd,
                deposit_amount,
                deposit_value_usd,
                ft_supply_amount,
            )?;
            msg!(
                "Mint Fund tokens to the user. ft_to_mint: {}, ft_supply_amount: {}, current_assets_usd: {}",
                ft_to_mint, ft_supply_amount,
                current_assets_usd
            );
            if ft_to_mint == 0 {
                msg!("Error: Deposit instruction didn't result in Fund tokens mint");
                return Err(ProgramError::Custom(170));
            }
            common::check_fund_token_mint(fund, fund_token_mint)?;
            let seeds: &[&[&[u8]]] = &[&[
                b"fund_authority",
                fund.name.as_bytes(),
                &[fund.authority_bump],
            ]];
            pda::mint_to_with_seeds(
                user_fund_token_account,
                fund_token_mint,
                fund_authority,
                seeds,
                ft_to_mint,
            )?;

            // update stats
            msg!("Update Fund stats");
            fund_info.set_amount_invested_usd(
                fund_info.get_amount_invested_usd()? + deposit_value_usd,
            )?;
            fund_info.set_current_assets_usd(current_assets_usd + deposit_value_usd)?;

            msg!("Update user stats");
            user_info.last_deposit.time = clock::get_time()?;
            user_info.last_deposit.amount = deposit_amount;
            user_info.deposit_request.time = 0;
            user_info.deposit_request.amount = 0;
        } else {
            // record the Fund as a delegate for the specified token amount to have tokens deposited later upon approval
            msg!(
                "Approve Fund as a delegate for {} tokens. deposit_fee: {}, deposit_value_usd: {}",
                amount_with_fee,
                deposit_fee,
                deposit_value_usd
            );
            account::approve_delegate(
                user_deposit_token_account,
                fund_authority,
                user_account,
                amount_with_fee,
            )?;
            user_info.deposit_request.time = clock::get_time()?;
            user_info.deposit_request.amount = amount_with_fee;
        }

        user_info.deny_reason = ArrayString64::default();
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
