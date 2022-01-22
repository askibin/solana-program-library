//! Cancel withdrawal from the Fund instruction handler

use {
    crate::common,
    solana_farm_sdk::{
        fund::{Fund, FundUserInfo},
        program::account,
        string::ArrayString64,
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn cancel_withdrawal(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [user_account, _fund_metadata, _fund_info_account, _spl_token_program, user_info_account, user_fund_token_account, custody_token_metadata] =
        accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let custody_token =
            if let Ok(token) = Token::unpack(&custody_token_metadata.try_borrow_data()?) {
                token
            } else {
                msg!("Failed to load custody token metadata");
                return Err(ProgramError::InvalidAccountData);
            };
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
            msg!("No pending withdrawals found");
            return Ok(());
        }

        msg!("Cancel pending withdrawal");
        account::revoke_delegate(user_fund_token_account, user_account)?;
        user_info.withdrawal_request.time = 0;
        user_info.withdrawal_request.amount = 0;
        user_info.deny_reason = ArrayString64::default();
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
