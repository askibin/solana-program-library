//! Fund entrypoint.

#![cfg(not(feature = "no-entrypoint"))]

use {
    crate::{
        fund_info::FundInfo,
        instructions::{
            add_custody::add_custody, approve_deposit::approve_deposit,
            approve_withdrawal::approve_withdrawal, cancel_deposit::cancel_deposit,
            cancel_withdrawal::cancel_withdrawal, deny_deposit::deny_deposit,
            deny_withdrawal::deny_withdrawal, disable_deposits::disable_deposits,
            disable_withdrawals::disable_withdrawals, init::init, lock_assets::lock_assets,
            raydium_swap::raydium_swap, remove_custody::remove_custody,
            request_deposit::request_deposit, request_withdrawal::request_withdrawal,
            set_assets_tracking_config::set_assets_tracking_config,
            set_deposit_schedule::set_deposit_schedule,
            set_withdrawal_schedule::set_withdrawal_schedule, unlock_assets::unlock_assets,
            update_assets_with_custody::update_assets_with_custody, user_init::user_init,
        },
    },
    solana_farm_sdk::{
        fund::Fund,
        id::main_router,
        instruction::{amm::AmmInstruction, fund::FundInstruction},
        log::sol_log_params_short,
        program::pda,
        refdb,
        string::ArrayString64,
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
            request_withdrawal(&fund, accounts, amount)?
        }
        FundInstruction::CancelWithdrawal => {
            log_start("CancelWithdrawal", &fund.name);
            cancel_withdrawal(&fund, accounts)?
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
            disable_deposits(&fund, &mut FundInfo::new(fund_info_account), accounts)?
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
            set_withdrawal_schedule(
                &fund,
                &mut FundInfo::new(fund_info_account),
                accounts,
                &schedule,
            )?
        }
        FundInstruction::DisableWithdrawals => {
            log_start("DisableWithdrawals", &fund.name);
            check_authority(user_account, &fund)?;
            disable_withdrawals(&fund, &mut FundInfo::new(fund_info_account), accounts)?
        }
        FundInstruction::ApproveWithdrawal { amount } => {
            log_start("ApproveWithdrawal", &fund.name);
            check_authority(user_account, &fund)?;
            approve_withdrawal(&fund, accounts, amount)?
        }
        FundInstruction::DenyWithdrawal { deny_reason } => {
            log_start("DenyWithdrawal", &fund.name);
            check_authority(user_account, &fund)?;
            deny_withdrawal(&fund, accounts, &deny_reason)?
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
            update_assets_with_custody(&fund, accounts)?
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
        FundInstruction::RemoveCustody {
            target_hash,
            custody_type,
        } => {
            log_start("RemoveCustody", &fund.name);
            check_authority(user_account, &fund)?;
            remove_custody(&fund, accounts, target_hash, custody_type)?
        }
        FundInstruction::AmmInstructionRaydium { instruction } => match instruction {
            AmmInstruction::Swap {
                token_a_amount_in,
                token_b_amount_in,
                min_token_amount_out,
            } => {
                log_start("SwapRaydium", &fund.name);
                check_authority(user_account, &fund)?;
                raydium_swap(
                    &fund,
                    accounts,
                    token_a_amount_in,
                    token_b_amount_in,
                    min_token_amount_out,
                )?
            }
            _ => {
                msg!("Error: Unimplemented");
                return Err(ProgramError::InvalidArgument);
            }
        },
    }

    log_end(&fund.name);
    Ok(())
}
