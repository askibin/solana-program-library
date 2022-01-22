//! Solana Farm Client Fund Instructions

use {
    crate::error::FarmClientError,
    arrayref::array_ref,
    solana_farm_sdk::{
        fund::{
            FundAssetType, FundAssetsTrackingConfig, FundCustodyType, FundSchedule, OracleType,
        },
        instruction::{amm::AmmInstruction, fund::FundInstruction},
        string::str_to_as64,
    },
    solana_sdk::{
        hash::Hasher,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program, sysvar,
    },
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new Fund Init Instruction
    pub fn new_instruction_init_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        step: u64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;

        // fill in accounts and instruction data
        let data = FundInstruction::Init { step }.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(fund.fund_authority, false),
            AccountMeta::new_readonly(fund.fund_program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(fund.fund_token_ref, false),
            AccountMeta::new(fund.vaults_assets_info, false),
            AccountMeta::new(fund.custodies_assets_info, false),
            AccountMeta::new(fund.liquidation_state, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for initializing a new User for the Fund
    pub fn new_instruction_user_init_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(wallet_address, fund_name, token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::UserInit.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new set fund assets tracking config Instruction
    pub fn new_instruction_set_fund_assets_tracking_config(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        config: &FundAssetsTrackingConfig,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::SetAssetsTrackingConfig { config: *config }.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for adding a new custody to the Fund
    pub fn new_instruction_add_fund_custody(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;

        // get custodies
        let custodies = self.get_fund_custodies(fund_name)?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, custody_type)?;
        let fund_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, custody_type)?;
        let custody_fees_token_account =
            self.get_fund_custody_fees_token_account(fund_name, token_name, custody_type)?;
        let pyth_price_info =
            self.get_oracle_price_account(&(token_name.to_string() + "/USD"), OracleType::Pyth)?;

        // instruction params
        let custody_id = if custodies.is_empty() {
            0
        } else if custodies.last().unwrap().custody_id < u32::MAX {
            custodies.last().unwrap().custody_id + 1
        } else {
            return Err(FarmClientError::ValueError(
                "Number of custodies are over the limit".to_string(),
            ));
        };

        let current_hash = self
            .get_fund_assets(fund_name, FundAssetType::Custody)?
            .current_hash;
        let mut hasher = Hasher::default();
        let mut input = current_hash.to_le_bytes().to_vec();
        input.extend_from_slice(custody_metadata.as_ref());
        hasher.hash(input.as_slice());
        let hash = hasher.result();
        let target_hash = u64::from_le_bytes(*array_ref!(hash.as_ref(), 0, 8));

        // fill in accounts and instruction data
        let data = FundInstruction::AddCustody {
            target_hash,
            custody_id,
            custody_type,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new(token.mint, false),
            AccountMeta::new_readonly(pyth_price_info, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for removing the custody from the Fund
    pub fn new_instruction_remove_fund_custody(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;

        // get custodies
        let custodies = self.get_fund_custodies(fund_name)?;
        if custodies.is_empty() {
            return Err(FarmClientError::ValueError(
                "No active custodies found".to_string(),
            ));
        }
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, custody_type)?;
        let fund_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, custody_type)?;
        let custody_fees_token_account =
            self.get_fund_custody_fees_token_account(fund_name, token_name, custody_type)?;

        // instruction params
        let mut hasher = Hasher::default();
        for custody in custodies {
            if custody.address != custody_token_account {
                hasher.hash(custody.address.as_ref());
            }
        }
        let hash = hasher.result();
        let target_hash = u64::from_le_bytes(*array_ref!(hash.as_ref(), 0, 8));

        // fill in accounts and instruction data
        let data = FundInstruction::RemoveCustody {
            target_hash,
            custody_type,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new set deposit schedule Instruction
    pub fn new_instruction_set_fund_deposit_schedule(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        schedule: &FundSchedule,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::SetDepositSchedule {
            schedule: *schedule,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for disabling deposits to the Fund
    pub fn new_instruction_disable_deposits_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DisableDeposits.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for requesting deposit to the Fund
    pub fn new_instruction_request_deposit_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(wallet_address, fund_name, token_name)?;
        let user_deposit_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(wallet_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let pyth_price_info =
            self.get_oracle_price_account(&(token_name.to_string() + "/USD"), OracleType::Pyth)?;

        // fill in accounts and instruction data
        let data = FundInstruction::RequestDeposit {
            amount: self.to_token_amount(ui_amount, &token),
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_deposit_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(pyth_price_info, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for canceling pending deposit to the Fund
    pub fn new_instruction_cancel_deposit_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(wallet_address, fund_name, token_name)?;
        let user_deposit_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;

        // fill in accounts and instruction data
        let data = FundInstruction::CancelDeposit.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_deposit_token_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for approving deposit to the Fund
    pub fn new_instruction_approve_deposit_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(user_address, fund_name, token_name)?;
        let user_deposit_token_account =
            self.get_associated_token_address(user_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(user_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let pyth_price_info =
            self.get_oracle_price_account(&(token_name.to_string() + "/USD"), OracleType::Pyth)?;

        // fill in accounts and instruction data
        let data = FundInstruction::ApproveDeposit {
            amount: self.to_token_amount(ui_amount, &token),
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_deposit_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(pyth_price_info, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for denying deposit to the Fund
    pub fn new_instruction_deny_deposit_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        deny_reason: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(user_address, fund_name, token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DenyDeposit {
            deny_reason: str_to_as64(deny_reason)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new set withdrawal schedule Instruction
    pub fn new_instruction_set_fund_withdrawal_schedule(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        schedule: &FundSchedule,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::SetWithdrawalSchedule {
            schedule: *schedule,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for disabling withdrawals from the Fund
    pub fn new_instruction_disable_withdrawals_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DisableWithdrawals.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for requesting withdrawal from the Fund
    pub fn new_instruction_request_withdrawal_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(wallet_address, fund_name, token_name)?;
        let user_withdrawal_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(wallet_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let pyth_price_info =
            self.get_oracle_price_account(&(token_name.to_string() + "/USD"), OracleType::Pyth)?;

        // fill in accounts and instruction data
        let data = FundInstruction::RequestWithdrawal {
            amount: self.to_token_amount(ui_amount, &fund_token),
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_withdrawal_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(pyth_price_info, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for canceling pending withdrawal from the Fund
    pub fn new_instruction_cancel_withdrawal_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(wallet_address, fund_name, token_name)?;
        let user_withdrawal_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;

        // fill in accounts and instruction data
        let data = FundInstruction::CancelWithdrawal.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_withdrawal_token_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for approving withdrawal from the Fund
    pub fn new_instruction_approve_withdrawal_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(user_address, fund_name, token_name)?;
        let user_withdrawal_token_account =
            self.get_associated_token_address(user_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(user_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let pyth_price_info =
            self.get_oracle_price_account(&(token_name.to_string() + "/USD"), OracleType::Pyth)?;

        // fill in accounts and instruction data
        let data = FundInstruction::ApproveWithdrawal {
            amount: self.to_token_amount(ui_amount, &fund_token),
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_withdrawal_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(pyth_price_info, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for denying withdrawal from the Fund
    pub fn new_instruction_deny_withdrawal_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        deny_reason: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account =
            self.get_fund_user_info_account(user_address, fund_name, token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DenyWithdrawal {
            deny_reason: str_to_as64(deny_reason)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for moving deposited assets to the Fund
    pub fn new_instruction_lock_assets_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let wd_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let wd_custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let trading_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::Trading)?;
        let trading_custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, FundCustodyType::Trading)?;

        // fill in accounts and instruction data
        let data = FundInstruction::LockAssets {
            amount: self.to_token_amount(ui_amount, &token),
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(wd_custody_token_account, false),
            AccountMeta::new(wd_custody_metadata, false),
            AccountMeta::new(trading_custody_token_account, false),
            AccountMeta::new(trading_custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for releasing assets from the Fund to Deposit/Withdraw custody
    pub fn new_instruction_unlock_assets_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let wd_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let wd_custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let trading_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::Trading)?;
        let trading_custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, FundCustodyType::Trading)?;

        // fill in accounts and instruction data
        let data = FundInstruction::UnlockAssets {
            amount: self.to_token_amount(ui_amount, &token),
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(wd_custody_token_account, false),
            AccountMeta::new(wd_custody_metadata, false),
            AccountMeta::new(trading_custody_token_account, false),
            AccountMeta::new(trading_custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for tokens swap
    pub fn new_instruction_fund_swap(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        protocol: &str,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        let pool_swap_inst = self.new_instruction_swap(
            &fund.fund_authority,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )?;

        let data = match protocol {
            "RDM" => FundInstruction::AmmInstructionRaydium {
                instruction: AmmInstruction::unpack(pool_swap_inst.data.as_slice())?,
            }
            .to_vec()?,
            _ => {
                return Err(FarmClientError::ValueError(
                    format!("Unsupported protocol {} for Fund {}", protocol, fund_name).to_string(),
                ));
            }
        };

        let mut accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
        ];
        accounts.extend_from_slice(&pool_swap_inst.accounts[1..]);

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for adding a new custody to the Fund
    pub fn new_instruction_update_fund_assets_with_custody(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;

        // get custodies
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, custody_type)?;
        let fund_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, custody_type)?;

        // fill in accounts and instruction data
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data: FundInstruction::UpdateAssetsWithCustody.to_vec()?,
            accounts,
        })
    }
}
