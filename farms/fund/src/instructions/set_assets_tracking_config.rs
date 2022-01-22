//! Fund SetDepositSchedule instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        fund::{Fund, FundAssetsTrackingConfig},
        instruction::fund::FundInstruction,
        program::{clock, pda},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn set_assets_tracking_config(
    fund: &Fund,
    fund_info: &mut FundInfo,
    accounts: &[AccountInfo],
    config: &FundAssetsTrackingConfig,
) -> ProgramResult {
    msg!("Update Fund assets tracking parameters");
    fund_info.set_assets_limit_usd(config.assets_limit_usd)?;
    fund_info.set_assets_max_update_age_sec(config.max_update_age_sec)?;
    fund_info.set_assets_max_price_error(config.max_price_error)?;
    fund_info.set_assets_max_price_age_sec(config.max_price_age_sec)?;

    msg!("Update Fund stats");
    fund_info.update_admin_action_time()
}
