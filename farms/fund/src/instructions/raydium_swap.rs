//! Swap tokens with the Raydium pool instruction

use {
    solana_farm_sdk::fund::Fund,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult},
};

pub fn raydium_swap(
    _fund: &Fund,
    _accounts: &[AccountInfo],
    _token_a_amount_in: u64,
    _token_b_amount_in: u64,
    _min_token_amount_out: u64,
) -> ProgramResult {
    Ok(())
}
