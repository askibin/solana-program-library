//! Fund entrypoint.

#![cfg(not(feature = "no-entrypoint"))]

use {
    crate::{
        fund_info::FundInfo,
        instructions::{
            /*lock_assets::lock_assets, */ add_custody::add_custody,
            approve_deposit::approve_deposit, cancel_deposit::cancel_deposit,
            deny_deposit::deny_deposit, init::init, lock_assets::lock_assets,
            request_deposit::request_deposit,
            set_assets_tracking_config::set_assets_tracking_config,
            set_deposit_schedule::set_deposit_schedule, unlock_assets::unlock_assets,
            user_init::user_init,
        },
    },
    solana_farm_sdk::{
        fund::Fund, id::main_router, instruction::fund::FundInstruction, log::sol_log_params_short,
        program::pda, refdb, string::ArrayString64,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        log::sol_log_compute_units,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

fn log_start(instruction: &str, fund_name: &ArrayString64) {
    msg!(
        "Processing FundInstruction::{} for {}",
        instruction,
        fund_name.as_str()
    );
    sol_log_compute_units();
}

fn log_end(fund_name: &ArrayString64) {
    sol_log_compute_units();
    msg!("Fund {} end of instruction", fund_name.as_str());
}

fn check_authority(user_account: &AccountInfo, fund: &Fund) -> ProgramResult {
    if user_account.key != &fund.admin_account {
        msg!(
            "Error: Instruction must be performed by the admin {}",
            fund.admin_account
        );
        Err(ProgramError::IllegalOwner)
    } else if !user_account.is_signer {
        Err(ProgramError::MissingRequiredSignature)
    } else {
        Ok(())
    }
}

entrypoint!(process_instruction);
/// Program's entrypoint.
///
/// # Arguments
/// * `program_id` - Public key of the fund.
/// * `accounts` - Accounts, see handlers in particular strategy for the list.
/// * `instructions_data` - Packed FundInstruction.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Fund entrypoint");
    if cfg!(feature = "debug") {
        sol_log_params_short(accounts, instruction_data);
    }

    let account_info_iter = &mut accounts.iter();
    let user_account = next_account_info(account_info_iter)?;
    let fund_metadata = next_account_info(account_info_iter)?;
    let fund_info_account = next_account_info(account_info_iter)?;

    // unpack Fund's metadata and validate Fund accounts
    let fund = Fund::unpack(&fund_metadata.try_borrow_data()?)?;
    let derived_fund_metadata =
        pda::find_target_pda_with_bump(refdb::StorageType::Fund, &fund.name, fund.metadata_bump)?;
    if &fund.info_account != fund_info_account.key
        || &derived_fund_metadata != fund_metadata.key
        || fund_metadata.owner != &main_router::id()
    {
        msg!("Error: Invalid Fund accounts");
        return Err(ProgramError::InvalidArgument);
    }
    if &fund.fund_program_id != program_id {
        msg!("Error: Invalid Fund program id");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Read and unpack instruction data
    let instruction = FundInstruction::unpack(instruction_data)?;

    match instruction {
        FundInstruction::UserInit => {
            log_start("UserInit", &fund.name);
            user_init(&fund, accounts)?
        }
        FundInstruction::RequestDeposit { amount } => {
            log_start("RequestDeposit", &fund.name);
            request_deposit(&fund, accounts, amount)?
        }
        FundInstruction::CancelDeposit => {
            log_start("CancelDeposit", &fund.name);
            cancel_deposit(&fund, accounts)?
        }
        FundInstruction::RequestWithdrawal { amount } => {
            log_start("RequestWithdrawal", &fund.name);
        }
        FundInstruction::CancelWithdrawal => {
            log_start("CancelWithdrawal", &fund.name);
        }
        FundInstruction::Init { step } => {
            log_start("Init", &fund.name);
            check_authority(user_account, &fund)?;
            init(&fund, accounts, step)?
        }
        FundInstruction::SetDepositSchedule { schedule } => {
            log_start("SetDepositSchedule", &fund.name);
            check_authority(user_account, &fund)?;
            set_deposit_schedule(
                &fund,
                &mut FundInfo::new(fund_info_account),
                accounts,
                &schedule,
            )?
        }
        FundInstruction::DisableDeposits => {
            log_start("DisableDeposits", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::ApproveDeposit { amount } => {
            log_start("ApproveDeposit", &fund.name);
            check_authority(user_account, &fund)?;
            approve_deposit(&fund, accounts, amount)?
        }
        FundInstruction::DenyDeposit { deny_reason } => {
            log_start("DenyDeposit", &fund.name);
            check_authority(user_account, &fund)?;
            deny_deposit(&fund, accounts, &deny_reason)?
        }
        FundInstruction::SetWithdrawalSchedule { schedule } => {
            log_start("SetWithdrawalSchedule", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::DisableWithdrawals => {
            log_start("DisableWithdrawals", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::ApproveWithdrawal { amount } => {
            log_start("ApproveWithdrawal", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::DenyWithdrawal { deny_reason } => {
            log_start("DenyWithdrawal", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::LockAssets { amount } => {
            log_start("LockAssets", &fund.name);
            check_authority(user_account, &fund)?;
            lock_assets(&fund, accounts, amount)?
        }
        FundInstruction::UnlockAssets { amount } => {
            log_start("UnlockAssets", &fund.name);
            check_authority(user_account, &fund)?;
            unlock_assets(&fund, accounts, amount)?
        }
        FundInstruction::SetAssetsTrackingConfig { config } => {
            log_start("SetAssetsTrackingConfig", &fund.name);
            check_authority(user_account, &fund)?;
            set_assets_tracking_config(
                &fund,
                &mut FundInfo::new(fund_info_account),
                accounts,
                &config,
            )?
        }
        FundInstruction::UpdateAssetsWithVault => {
            log_start("UpdateAssetsWithVault", &fund.name);
        }
        FundInstruction::UpdateAssetsWithCustody => {
            log_start("UpdateAssetsWithCustody", &fund.name);
        }
        FundInstruction::AddVault => {
            log_start("AddVault", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::RemoveVault => {
            log_start("RemoveVault", &fund.name);
            check_authority(user_account, &fund)?;
        }
        FundInstruction::AddCustody {
            target_hash,
            custody_id,
            custody_type,
        } => {
            log_start("AddCustody", &fund.name);
            check_authority(user_account, &fund)?;
            add_custody(&fund, accounts, target_hash, custody_id, custody_type)?
        }
        FundInstruction::RemoveCustody => {
            log_start("RemoveCustody", &fund.name);
            check_authority(user_account, &fund)?;
        }
    }

    log_end(&fund.name);
    Ok(())
}
