//! Cancel deposit to the Fund instruction handler

use {
    crate::common,
    solana_farm_sdk::{
        fund::{Fund, FundUserInfo},
        id::main_router,
        program::account,
        string::ArrayString64,
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn cancel_deposit(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        _fund_metadata,
        _fund_info_account,
        _spl_token_program,
        user_info_account,
        user_deposit_token_account,
        custody_token_metadata
        ] = accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if custody_token_metadata.owner != &main_router::id() {
            msg!("Error: Invalid custody token metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        let mut user_info = account::unpack::<FundUserInfo>(user_info_account, "user info")?;
        common::check_user_info_account(
            fund,
            &custody_token,
            &user_info,
            user_account,
            user_info_account,
        )?;

        // check if there are any pending requests
        if user_info.deposit_request.amount == 0 {
            msg!("No pending deposits found");
            return Ok(());
        }

        // cancel pending deposit
        msg!("Cancel pending deposit");
        account::revoke_delegate(user_deposit_token_account, user_account)?;
        user_info.deposit_request.time = 0;
        user_info.deposit_request.amount = 0;
        user_info.deny_reason = ArrayString64::default();
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
