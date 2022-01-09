use {
    log::info,
    solana_farm_client::{client::FarmClient, error::FarmClientError},
    solana_farm_sdk::{
        farm::{FarmRoute, FarmType},
        fund::{Fund, FundType},
        git_token::GitToken,
        id::main_router_admin,
        program::pda::find_target_pda,
        refdb::StorageType,
        string::{str_to_as64, to_pretty_json, ArrayString64},
        token::{Token, TokenType},
        vault::{Vault, VaultStrategy, VaultType},
    },
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
    },
    std::collections::HashMap,
    std::str::FromStr,
};

pub fn init_fund(
    client: &FarmClient,
    keypair: &Keypair,
    fund_name: Option<&str>,
    fund_token_name: Option<&str>,
) -> Result<String, FarmClientError> {
    let rand_name = "FUND_".to_string() + &rand::random::<u32>().to_string();
    let fund_name: &str = if let Some(name) = fund_name {
        name
    } else {
        &rand_name
    };

    let fund_token_name = if let Some(name) = fund_token_name {
        name
    } else {
        fund_name
    };

    let fund_address = if let Ok(address) = client.get_program_id(fund_name) {
        address
    } else {
        Pubkey::from_str("CivDUhF9Vkar9jJjwxDwdefqSd7nYzyMY8wbrGYax5hQ").unwrap()
    };

    if client.get_token(fund_token_name).is_err() {
        let last_index = client.get_refdb_last_index(&StorageType::Token.to_string())?;
        let token = Token {
            name: str_to_as64(fund_token_name)?,
            description: str_to_as64(&(fund_name.to_string() + " Token"))?,
            token_type: TokenType::FundToken,
            refdb_index: Some(last_index),
            refdb_counter: 0u16,
            decimals: 6,
            chain_id: 101,
            mint: Pubkey::find_program_address(
                &[b"fund_token_mint", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
        };

        info!("Recording token {}", fund_token_name);
        client.add_token(keypair, token)?;
    }

    if client.get_fund(fund_name).is_err() {
        let last_index = client.get_refdb_last_index(&StorageType::Fund.to_string())?;
        let fund = Fund {
            name: str_to_as64(fund_name).unwrap(),
            description: ArrayString64::default(),
            version: 1,
            fund_type: FundType::General,
            official: true,
            refdb_index: Some(last_index),
            refdb_counter: 0u16,
            metadata_bump: find_target_pda(StorageType::Fund, &str_to_as64(fund_name).unwrap()).1,
            authority_bump: Pubkey::find_program_address(
                &[b"fund_authority", fund_name.as_bytes()],
                &fund_address,
            )
            .1,
            fund_token_bump: Pubkey::find_program_address(
                &[b"fund_token_mint", fund_name.as_bytes()],
                &fund_address,
            )
            .1,
            fund_program_id: fund_address,
            fund_authority: Pubkey::find_program_address(
                &[b"fund_authority", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            fund_token_ref: find_target_pda(
                StorageType::Token,
                &str_to_as64(fund_token_name).unwrap(),
            )
            .0,
            info_account: Pubkey::find_program_address(
                &[b"info_account", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            admin_account: main_router_admin::id(),
            vaults_assets_info: Pubkey::find_program_address(
                &[b"vaults_assets_info", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            custodies_assets_info: Pubkey::find_program_address(
                &[b"custodies_assets_info", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            liquidation_state: Pubkey::find_program_address(
                &[b"liquidation_state", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
        };

        info!("Recording fund {}", fund_name);
        client.add_fund(keypair, fund)?;
    }

    info!("Initializing fund {}", fund_name);
    client.init_fund(keypair, &fund_name, 0)?;

    Ok(fund_name.to_string())
}
