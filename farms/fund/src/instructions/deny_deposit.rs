//! Deny deposit to the Fund instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundUserInfo},
        string::ArrayString64,
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn deny_deposit(
    fund: &Fund,
    accounts: &[AccountInfo],
    deny_reason: &ArrayString64,
) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [_admin_account, _fund_metadata, fund_info_account, user_account, user_info_account, custody_token_metadata] =
        accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut user_info =
            if let Ok(user_info) = FundUserInfo::unpack(&user_info_account.try_borrow_data()?) {
                user_info
            } else {
                msg!("Failed to load user info account");
                return Err(ProgramError::InvalidAccountData);
            };
        let custody_token =
            if let Ok(token) = Token::unpack(&custody_token_metadata.try_borrow_data()?) {
                token
            } else {
                msg!("Failed to load custody token metadata");
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

        // update stats
        msg!("Update Fund stats");
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info.update_admin_action_time()?;

        // update user stats
        msg!("Update user stats");
        user_info.last_deposit.time = user_info.deposit_request.time;
        user_info.last_deposit.amount = user_info.deposit_request.amount;
        user_info.deposit_request.time = 0;
        user_info.deposit_request.amount = 0;
        user_info.deny_reason = *deny_reason;
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
