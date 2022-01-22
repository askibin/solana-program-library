//! Initialize a new user for the Fund instruction handler
use {
    solana_farm_sdk::{
        fund::{Fund, FundUserAction, FundUserInfo, DISCRIMINATOR_FUND_USER_INFO},
        program::pda,
        string::ArrayString64,
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn user_init(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    //#[allow(clippy::deprecated_cfg_attr)]
    //#[cfg_attr(rustfmt, rustfmt_skip)]
    if let [user_account, fund_metadata, _fund_info_account, user_info_account, custody_token_ref, _system_program] =
        accounts
    {
        msg!("Create user info account");
        let custody_token = Token::unpack(&custody_token_ref.try_borrow_data()?)?;
        let seeds: &[&[u8]] = &[
            b"user_info_account",
            custody_token.name.as_bytes(),
            user_account.key.as_ref(),
            fund.name.as_bytes(),
        ];
        let bump = Pubkey::find_program_address(seeds, &fund.fund_program_id).1;
        pda::init_system_account(
            user_account,
            user_info_account,
            &fund.fund_program_id,
            &fund.fund_program_id,
            seeds,
            FundUserInfo::LEN,
        )?;
        let user_info = FundUserInfo {
            discriminator: DISCRIMINATOR_FUND_USER_INFO,
            fund_ref: *fund_metadata.key,
            token_ref: *custody_token_ref.key,
            deposit_request: FundUserAction::default(),
            last_deposit: FundUserAction::default(),
            withdrawal_request: FundUserAction::default(),
            last_withdrawal: FundUserAction::default(),
            deny_reason: ArrayString64::default(),
            bump,
        };
        user_info.pack(*user_info_account.try_borrow_mut_data()?)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    Ok(())
}
