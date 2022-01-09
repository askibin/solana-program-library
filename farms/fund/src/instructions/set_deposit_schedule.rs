//! Fund SetDepositSchedule instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        fund::{Fund, FundSchedule},
        instruction::fund::FundInstruction,
        program::{clock, pda},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn set_deposit_schedule(
    fund: &Fund,
    fund_info: &mut FundInfo,
    accounts: &[AccountInfo],
    schedule: &FundSchedule,
) -> ProgramResult {
    msg!("Update fund deposit parameters");
    if schedule.start_time >= schedule.end_time {
        msg!("Error: start_time must be less than end_time");
        return Err(ProgramError::InvalidArgument);
    }

    fund_info.set_deposit_start_time(schedule.start_time)?;
    fund_info.set_deposit_end_time(schedule.end_time)?;
    fund_info.set_deposit_approval_required(schedule.approval_required)?;
    fund_info.set_deposit_limit_usd(schedule.limit_usd)?;
    fund_info.set_deposit_fee(schedule.fee)?;

    msg!("Update fund stats");
    fund_info.update_admin_action_time()
}
