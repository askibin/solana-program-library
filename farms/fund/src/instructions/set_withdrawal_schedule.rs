//! Fund SetWithdrawalSchedule instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::fund::{Fund, FundSchedule},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn set_withdrawal_schedule(
    _fund: &Fund,
    fund_info: &mut FundInfo,
    _accounts: &[AccountInfo],
    schedule: &FundSchedule,
) -> ProgramResult {
    msg!("Update Fund withdrawal parameters");
    if schedule.start_time >= schedule.end_time {
        msg!("Error: start_time must be less than end_time");
        return Err(ProgramError::InvalidArgument);
    }

    fund_info.set_withdrawal_start_time(schedule.start_time)?;
    fund_info.set_withdrawal_end_time(schedule.end_time)?;
    fund_info.set_withdrawal_approval_required(schedule.approval_required)?;
    fund_info.set_withdrawal_limit_usd(schedule.limit_usd)?;
    fund_info.set_withdrawal_fee(schedule.fee)?;

    msg!("Update Fund stats");
    fund_info.update_admin_action_time()
}
