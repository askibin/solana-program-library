//! Common functions for tests

use {
    solana_farm_client::client::FarmClient,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::keypair::read_keypair_file},
};

#[allow(dead_code)]
pub fn get_endpoint_and_keypair() -> (String, Keypair) {
    let cli_config = if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
        solana_cli_config::Config::load(config_file).unwrap()
    } else {
        solana_cli_config::Config::default()
    };

    (
        cli_config.json_rpc_url.to_string(),
        read_keypair_file(&cli_config.keypair_path).unwrap_or_else(|_| {
            panic!("Filed to read keypair from \"{}\"", cli_config.keypair_path)
        }),
    )
}
