//! Fund entrypoint.

#![cfg(not(feature = "no-entrypoint"))]

use {
    crate::{
        fund_info::FundInfo,
        instructions::{
            add_custody::add_custody, add_vault::add_vault, approve_deposit::approve_deposit,
            approve_withdrawal::approve_withdrawal, cancel_deposit::cancel_deposit,
            cancel_withdrawal::cancel_withdrawal, deny_deposit::deny_deposit,
            deny_withdrawal::deny_withdrawal, disable_deposits::disable_deposits,
            disable_withdrawals::disable_withdrawals, init::init, lock_assets::lock_assets,
            raydium, remove_custody::remove_custody, remove_vault::remove_vault,
            request_deposit::request_deposit, request_withdrawal::request_withdrawal,
            set_assets_tracking_config::set_assets_tracking_config,
            set_deposit_schedule::set_deposit_schedule,
            set_withdrawal_schedule::set_withdrawal_schedule, start_liquidation::start_liquidation,
            stop_liquidation::stop_liquidation, unlock_assets::unlock_assets,
            update_assets_with_custody::update_assets_with_custody,
            update_assets_with_vault::update_assets_with_vault, user_init::user_init,
            withdraw_fees::withdraw_fees,
        },
    },
    solana_farm_sdk::{
        fund::Fund,
        id::main_router,
        instruction::{amm::AmmInstruction, fund::FundInstruction, vault::VaultInstruction},
        log::sol_log_params_short,
        program::{account, pda},
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

fn check_admin_authority(user_account: &AccountInfo, fund: &Fund) -> ProgramResult {
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

fn check_manager_authority(user_account: &AccountInfo, fund: &Fund) -> ProgramResult {
    if user_account.key != &fund.fund_manager && user_account.key != &fund.admin_account {
        msg!("Error: Instruction must be performed by the fund manager or admin");
        msg!("Fund manager: {}", fund.fund_manager);
        msg!("Fund admin: {}", fund.admin_account);
        Err(ProgramError::IllegalOwner)
    } else if !user_account.is_signer {
        Err(ProgramError::MissingRequiredSignature)
    } else {
        Ok(())
    }
}

fn check_manager_authority_or_liquidation(
    user_account: &AccountInfo,
    fund_info_account: &AccountInfo,
    fund: &Fund,
) -> ProgramResult {
    if FundInfo::new(fund_info_account).get_liquidation_start_time()? > 0 {
        return Ok(());
    }
    check_manager_authority(user_account, fund)
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
    let fund = account::unpack::<Fund>(fund_metadata, "Fund")?;
    let derived_fund_metadata =
        pda::find_target_pda_with_bump(refdb::StorageType::Fund, &fund.name, fund.metadata_bump)?;
    if &fund.info_account != fund_info_account.key
        || &derived_fund_metadata != fund_metadata.key
        || fund_metadata.owner != &main_router::id()
    {
        msg!("Error: Invalid Fund accounts");
        return Err(ProgramError::Custom(511));
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
            check_admin_authority(user_account, &fund)?;
            init(&fund, accounts, step)?
        }
        FundInstruction::SetDepositSchedule { schedule } => {
            log_start("SetDepositSchedule", &fund.name);
            check_manager_authority(user_account, &fund)?;
            set_deposit_schedule(
                &fund,
                &mut FundInfo::new(fund_info_account),
                accounts,
                &schedule,
            )?
        }
        FundInstruction::DisableDeposits => {
            log_start("DisableDeposits", &fund.name);
            check_manager_authority(user_account, &fund)?;
            disable_deposits(&fund, &mut FundInfo::new(fund_info_account), accounts)?
        }
        FundInstruction::ApproveDeposit { amount } => {
            log_start("ApproveDeposit", &fund.name);
            check_manager_authority(user_account, &fund)?;
            approve_deposit(&fund, accounts, amount)?
        }
        FundInstruction::DenyDeposit { deny_reason } => {
            log_start("DenyDeposit", &fund.name);
            check_manager_authority(user_account, &fund)?;
            deny_deposit(&fund, accounts, &deny_reason)?
        }
        FundInstruction::SetWithdrawalSchedule { schedule } => {
            log_start("SetWithdrawalSchedule", &fund.name);
            check_manager_authority(user_account, &fund)?;
            set_withdrawal_schedule(
                &fund,
                &mut FundInfo::new(fund_info_account),
                accounts,
                &schedule,
            )?
        }
        FundInstruction::DisableWithdrawals => {
            log_start("DisableWithdrawals", &fund.name);
            check_manager_authority(user_account, &fund)?;
            disable_withdrawals(&fund, &mut FundInfo::new(fund_info_account), accounts)?
        }
        FundInstruction::ApproveWithdrawal { amount } => {
            log_start("ApproveWithdrawal", &fund.name);
            check_manager_authority(user_account, &fund)?;
            approve_withdrawal(&fund, accounts, amount)?
        }
        FundInstruction::DenyWithdrawal { deny_reason } => {
            log_start("DenyWithdrawal", &fund.name);
            check_manager_authority(user_account, &fund)?;
            deny_withdrawal(&fund, accounts, &deny_reason)?
        }
        FundInstruction::LockAssets { amount } => {
            log_start("LockAssets", &fund.name);
            check_manager_authority(user_account, &fund)?;
            lock_assets(&fund, accounts, amount)?
        }
        FundInstruction::UnlockAssets { amount } => {
            log_start("UnlockAssets", &fund.name);
            check_manager_authority_or_liquidation(user_account, fund_info_account, &fund)?;
            unlock_assets(&fund, accounts, amount)?
        }
        FundInstruction::SetAssetsTrackingConfig { config } => {
            log_start("SetAssetsTrackingConfig", &fund.name);
            check_admin_authority(user_account, &fund)?;
            set_assets_tracking_config(
                &fund,
                &mut FundInfo::new(fund_info_account),
                accounts,
                &config,
            )?
        }
        FundInstruction::UpdateAssetsWithVault => {
            log_start("UpdateAssetsWithVault", &fund.name);
            update_assets_with_vault(&fund, accounts)?
        }
        FundInstruction::UpdateAssetsWithCustody => {
            log_start("UpdateAssetsWithCustody", &fund.name);
            update_assets_with_custody(&fund, accounts)?
        }
        FundInstruction::AddVault {
            target_hash,
            vault_id,
            vault_type,
        } => {
            log_start("AddVault", &fund.name);
            check_admin_authority(user_account, &fund)?;
            add_vault(&fund, accounts, target_hash, vault_id, vault_type)?
        }
        FundInstruction::RemoveVault {
            target_hash,
            vault_type,
        } => {
            log_start("RemoveVault", &fund.name);
            check_admin_authority(user_account, &fund)?;
            remove_vault(&fund, accounts, target_hash, vault_type)?
        }
        FundInstruction::AddCustody {
            target_hash,
            custody_id,
            custody_type,
        } => {
            log_start("AddCustody", &fund.name);
            check_admin_authority(user_account, &fund)?;
            add_custody(&fund, accounts, target_hash, custody_id, custody_type)?
        }
        FundInstruction::RemoveCustody {
            target_hash,
            custody_type,
        } => {
            log_start("RemoveCustody", &fund.name);
            check_admin_authority(user_account, &fund)?;
            remove_custody(&fund, accounts, target_hash, custody_type)?
        }
        FundInstruction::StartLiquidation => {
            log_start("StartLiquidation", &fund.name);
            start_liquidation(&fund, accounts)?
        }
        FundInstruction::StopLiquidation => {
            log_start("StopLiquidation", &fund.name);
            check_admin_authority(user_account, &fund)?;
            stop_liquidation(&fund, accounts)?
        }
        FundInstruction::WithdrawFees { amount } => {
            log_start("WithdrawFees", &fund.name);
            withdraw_fees(&fund, accounts, amount)?
        }
        FundInstruction::AmmInstructionRaydium { instruction } => match instruction {
            AmmInstruction::UserInit => {
                log_start("UserInitRaydium", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::user_init::user_init(&fund, accounts)?
            }
            AmmInstruction::AddLiquidity {
                max_token_a_amount,
                max_token_b_amount,
            } => {
                log_start("AddLiquidityRaydium", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::add_liquidity::add_liquidity(
                    &fund,
                    accounts,
                    max_token_a_amount,
                    max_token_b_amount,
                )?
            }
            AmmInstruction::RemoveLiquidity { amount } => {
                log_start("RemoveLiquidityRaydium", &fund.name);
                check_manager_authority_or_liquidation(user_account, fund_info_account, &fund)?;
                raydium::remove_liquidity::remove_liquidity(&fund, accounts, amount)?
            }
            AmmInstruction::Swap {
                token_a_amount_in,
                token_b_amount_in,
                min_token_amount_out,
            } => {
                log_start("SwapRaydium", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::swap::swap(
                    &fund,
                    accounts,
                    token_a_amount_in,
                    token_b_amount_in,
                    min_token_amount_out,
                )?
            }
            AmmInstruction::Stake { amount } => {
                log_start("StakeRaydium", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::stake::stake(&fund, accounts, amount, false)?
            }
            AmmInstruction::Unstake { amount } => {
                log_start("UnstakeRaydium", &fund.name);
                check_manager_authority_or_liquidation(user_account, fund_info_account, &fund)?;
                raydium::unstake::unstake(&fund, accounts, amount)?
            }
            AmmInstruction::Harvest => {
                log_start("HarvestRaydium", &fund.name);
                check_manager_authority_or_liquidation(user_account, fund_info_account, &fund)?;
                raydium::stake::stake(&fund, accounts, 0, true)?
            }
            _ => {
                msg!("Error: Unimplemented");
                return Err(ProgramError::Custom(512));
            }
        },
        FundInstruction::VaultInstructionRaydium { instruction } => match instruction {
            VaultInstruction::AddLiquidity {
                max_token_a_amount,
                max_token_b_amount,
            } => {
                log_start("VaultAddLiquidity", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::vault_add_liquidity::add_liquidity(
                    &fund,
                    accounts,
                    max_token_a_amount,
                    max_token_b_amount,
                )?
            }
            VaultInstruction::LockLiquidity { amount } => {
                log_start("VaultLockLiquidity", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::vault_lock_liquidity::lock_liquidity(&fund, accounts, amount)?
            }
            VaultInstruction::UnlockLiquidity { amount } => {
                log_start("VaultUnlockLiquidity", &fund.name);
                check_manager_authority_or_liquidation(user_account, fund_info_account, &fund)?;
                raydium::vault_unlock_liquidity::unlock_liquidity(&fund, accounts, amount)?
            }
            VaultInstruction::RemoveLiquidity { amount } => {
                log_start("VaultRemoveLiquidity", &fund.name);
                check_manager_authority_or_liquidation(user_account, fund_info_account, &fund)?;
                raydium::vault_remove_liquidity::remove_liquidity(&fund, accounts, amount)?
            }
            VaultInstruction::UserInit {} => {
                log_start("VaultUserInit", &fund.name);
                check_manager_authority(user_account, &fund)?;
                raydium::vault_user_init::user_init(&fund, accounts)?
            }
            _ => {
                msg!("Error: Unimplemented");
                return Err(ProgramError::Custom(513));
            }
        },
    }

    log_end(&fund.name);
    Ok(())
}
