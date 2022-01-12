mod fixture;
mod utils;

use {
    log::info,
    solana_farm_client::{client::FarmClient, error::FarmClientError},
    solana_farm_sdk::{
        fund::{
            FundAssetsTrackingConfig, FundCustodyType, FundSchedule, OracleType,
            DISCRIMINATOR_FUND_CUSTODY, DISCRIMINATOR_FUND_USER_INFO,
        },
        string::str_to_as64,
    },
    solana_sdk::{
        clock::UnixTimestamp,
        commitment_config::{CommitmentConfig, CommitmentLevel},
        signature::Keypair,
        signer::Signer,
    },
};

fn get_time() -> UnixTimestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as UnixTimestamp
}

#[test]
// Runs all integration tests. Default config should have rpc url set to
// localhost or devnet and kepair_path should point to the admin keypair.
fn run_tests() -> Result<(), FarmClientError> {
    solana_logger::setup_with_default("main=debug,solana=debug");

    let (endpoint, admin_keypair) = utils::get_endpoint_and_keypair();
    let user_keypair = Keypair::new();
    let client = FarmClient::new_with_commitment(&endpoint, CommitmentConfig::confirmed());
    let wallet = user_keypair.pubkey();
    let fund_name = fixture::init_fund(&client, &admin_keypair, None, None)?;
    let fund = client.get_fund(&fund_name)?;
    let fund_token = client.get_token_by_ref(&fund.fund_token_ref)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    println!("{:#?}", fund_info);

    // init user for SOL deposit
    info!("Init user");
    let token_name = "SOL";
    assert!(client
        .get_fund_user_info(&wallet, &fund_name, token_name)
        .is_err());
    for i in 0..2 {
        client.confirm_async_transaction(
            &client
                .rpc_client
                .request_airdrop(&wallet, client.ui_amount_to_tokens(2.0, "SOL")?)?,
            CommitmentLevel::Confirmed,
        )?;
    }
    client.user_init_fund(&user_keypair, &fund_name, token_name)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    println!("{:#?}", user_info);
    assert_eq!(user_info.discriminator, DISCRIMINATOR_FUND_USER_INFO);

    // init SOL custody
    // deposit should fail while custody is missing
    info!("Init Deposit/Withdraw custody for SOL");
    assert!(client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::DepositWithdraw)
        .is_err());

    client.add_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let custody =
        client.get_fund_custody(&fund_name, token_name, FundCustodyType::DepositWithdraw)?;
    println!("{:#?}", custody);
    assert_eq!(custody.discriminator, DISCRIMINATOR_FUND_CUSTODY);
    assert_eq!(custody.custody_type, FundCustodyType::DepositWithdraw);

    // set assets tracking config
    info!("Set assets tracking config");
    let config = FundAssetsTrackingConfig {
        assets_limit_usd: 1000.0,
        max_update_age_sec: 600,
        max_price_error: 0.1,
        max_price_age_sec: 600,
    };
    client.set_fund_assets_tracking_config(&admin_keypair, &fund_name, &config)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.assets_config, config);

    // set deposit schedule
    info!("Set deposit schedule");
    assert!(client
        .request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)
        .is_err());
    let schedule = FundSchedule {
        start_time: 0,
        end_time: get_time() + 600,
        approval_required: true,
        limit_usd: client.get_oracle_price("SOL/USD", OracleType::Pyth, 0, 0.0)? * 1.5,
        fee: 0.01,
    };
    client.set_fund_deposit_schedule(&admin_keypair, &fund_name, &schedule)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.deposit_schedule, schedule);

    // request deposit
    info!("Request deposit over the limit");
    assert!(client
        .request_deposit_fund(&user_keypair, &fund_name, token_name, 1.8)
        .is_err());
    info!("Request deposit");
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(
        user_info.deposit_request.amount,
        client.ui_amount_to_tokens(1.123, "SOL")?
    );
    assert!(user_info.deposit_request.time > 0);
    assert!(user_info.deny_reason.is_empty());
    assert_eq!(
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?,
        0.0
    );

    // cancel deposit
    info!("Cancel deposit");
    client.cancel_deposit_fund(&user_keypair, &fund_name, token_name)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(user_info.deposit_request.amount, 0);
    assert_eq!(user_info.deposit_request.time, 0);
    assert!(user_info.deny_reason.is_empty());

    // request and deny
    info!("Request a new deposit and deny");
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)?;
    client.deny_deposit_fund(&admin_keypair, &wallet, &fund_name, token_name, "test")?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(user_info.deposit_request.amount, 0);
    assert_eq!(user_info.deposit_request.time, 0);
    assert_eq!(user_info.deny_reason, str_to_as64("test")?);
    assert_eq!(
        user_info.last_deposit.amount,
        client.ui_amount_to_tokens(1.123, "SOL")?
    );
    assert!(user_info.last_deposit.time > 0);

    // request and approve
    info!("Request a new deposit and approve");
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)?;
    client.approve_deposit_fund(&admin_keypair, &wallet, &fund_name, token_name, 0.123)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(user_info.deposit_request.amount, 0);
    assert_eq!(user_info.deposit_request.time, 0);
    assert!(user_info.deny_reason.is_empty());
    let deposited_amount = client.ui_amount_to_tokens(0.123 - 0.123 * 0.01, "SOL")?;
    assert_eq!(user_info.last_deposit.amount, deposited_amount);
    assert!(user_info.last_deposit.time > 0);
    let fund_token_balance = client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    assert!(fund_token_balance > 0.0);
    assert!(client.get_token_account_balance(&wallet, "SOL")? > 0.0);
    let wd_custody_token_address = client.get_fund_custody_token_account(
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let wd_fees_custody_token_address = client.get_fund_custody_fees_token_account(
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    assert_eq!(
        deposited_amount,
        client
            .rpc_client
            .get_token_account_balance(&wd_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );
    assert_eq!(
        client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount,
        client
            .rpc_client
            .get_token_account_balance(&wd_fees_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );

    // init second user
    let user_keypair2 = Keypair::new();
    let wallet2 = user_keypair2.pubkey();
    client.confirm_async_transaction(
        &client
            .rpc_client
            .request_airdrop(&wallet2, client.ui_amount_to_tokens(2.0, "SOL")?)?,
        CommitmentLevel::Confirmed,
    )?;

    // turn off approval requirement
    let schedule = FundSchedule {
        start_time: 0,
        end_time: get_time() + 600,
        approval_required: false,
        limit_usd: client.get_oracle_price("SOL/USD", OracleType::Pyth, 0, 0.0)? * 1.5,
        fee: 0.01,
    };
    client.set_fund_deposit_schedule(&admin_keypair, &fund_name, &schedule)?;

    // request instant deposit
    info!("Request instant deposit");
    client.request_deposit_fund(&user_keypair2, &fund_name, token_name, 0.123)?;
    let user_info = client.get_fund_user_info(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_info.deposit_request.amount, 0);
    assert_eq!(user_info.deposit_request.time, 0);
    assert!(user_info.deny_reason.is_empty());
    assert!(user_info.last_deposit.amount > 0);
    assert!(user_info.last_deposit.time > 0);
    let fund_token_balance2 =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert!(fund_token_balance2 > 0.0);
    // some tolerence needed due to potential SOL/USD price change
    assert!((fund_token_balance2 - fund_token_balance).abs() / fund_token_balance < 0.01);
    assert_eq!(
        deposited_amount * 2,
        client
            .rpc_client
            .get_token_account_balance(&wd_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );
    assert_eq!(
        (client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2,
        client
            .rpc_client
            .get_token_account_balance(&wd_fees_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );

    // init SOL trading custody
    // accept should fail while custody is missing
    info!("Init Trading custody for SOL");
    assert!(client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::Trading)
        .is_err());

    client.add_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::Trading,
    )?;
    let custody = client.get_fund_custody(&fund_name, token_name, FundCustodyType::Trading)?;
    println!("{:#?}", custody);
    assert_eq!(custody.discriminator, DISCRIMINATOR_FUND_CUSTODY);
    assert_eq!(custody.custody_type, FundCustodyType::Trading);

    // accept funds into trading custody
    info!("Accept funds into trading custody");
    client.lock_assets_fund(&admin_keypair, &fund_name, token_name, 0.0)?;
    assert_eq!(
        0,
        client
            .rpc_client
            .get_token_account_balance(&wd_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );
    let trading_custody_token_address =
        client.get_fund_custody_token_account(&fund_name, token_name, FundCustodyType::Trading)?;
    assert_eq!(
        deposited_amount * 2,
        client
            .rpc_client
            .get_token_account_balance(&trading_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );

    // release funds into w/d custody
    info!("Release funds into w/d custody");
    client.unlock_assets_fund(&admin_keypair, &fund_name, token_name, 0.0)?;
    assert_eq!(
        0,
        client
            .rpc_client
            .get_token_account_balance(&trading_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );
    assert_eq!(
        deposited_amount * 2,
        client
            .rpc_client
            .get_token_account_balance(&wd_custody_token_address)?
            .amount
            .parse::<u64>()
            .unwrap()
    );

    Ok(())
}
