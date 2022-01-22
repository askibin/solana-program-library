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

#[test]
// Runs all integration tests. Default config should have rpc url set to
// localhost or devnet and kepair_path should point to the admin keypair.
fn run_tests() -> Result<(), FarmClientError> {
    solana_logger::setup_with_default("main=debug,solana=debug");

    let (endpoint, admin_keypair) = utils::get_endpoint_and_keypair();
    let user_keypair = Keypair::new();
    let client = FarmClient::new_with_commitment(&endpoint, CommitmentConfig::confirmed());
    let wallet = user_keypair.pubkey();
    let fund_name = "TestFund5".to_string(); //fixture::init_fund(&client, &admin_keypair, Some("TestFund5"), None)?;
    let fund = client.get_fund(&fund_name)?;
    let fund_token = client.get_token_by_ref(&fund.fund_token_ref)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    println!("{:#?}", fund_info);

    // swap
    let token_a = "COIN";
    let token_b = "PC";
    let swap_amount = 0.01;

    info!(
        "{}",
        client.update_fund_assets_with_custody(
            &admin_keypair,
            &fund_name,
            "SOL",
            FundCustodyType::Trading
        )?
    );

    if client
        .get_fund_custody(&fund_name, token_a, FundCustodyType::Trading)
        .is_err()
    {
        info!("Init trading custody for {}", token_a);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_a,
            FundCustodyType::Trading,
        )?;
    }

    if client
        .get_fund_custody(&fund_name, token_a, FundCustodyType::DepositWithdraw)
        .is_err()
    {
        info!("Init deposit custody for {}", token_a);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_a,
            FundCustodyType::DepositWithdraw,
        )?;
    }

    let trading_custody_token_a_address =
        client.get_fund_custody_token_account(&fund_name, token_a, FundCustodyType::Trading)?;
    let trading_custody_token_b_address =
        client.get_fund_custody_token_account(&fund_name, token_b, FundCustodyType::Trading)?;
    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);

    if trading_custody_token_a_balance < swap_amount {
        info!("Set new deposit schedule");
        let schedule = FundSchedule {
            start_time: 0,
            end_time: utils::get_time() + 600,
            approval_required: false,
            limit_usd: f64::MAX,
            fee: 0.01,
        };
        client.set_fund_deposit_schedule(&admin_keypair, &fund_name, &schedule)?;
        info!("Deposit {} to the Fund", token_a);
        client.request_deposit_fund(
            &admin_keypair,
            &fund_name,
            token_a,
            swap_amount + swap_amount * 0.02,
        )?;
        info!("Move {} to trading custody", token_a);
        client.lock_assets_fund(&admin_keypair, &fund_name, token_a, 0.0)?;
    }

    info!(
        "{}",
        client.fund_swap(
            &admin_keypair,
            &fund_name,
            "RDM",
            token_a,
            token_b,
            swap_amount,
            0.0
        )?
    );
    let trading_custody_token_a_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    assert_eq!(
        trading_custody_token_a_balance - trading_custody_token_a_balance2,
        swap_amount
    );
    assert!(trading_custody_token_b_balance2 > trading_custody_token_b_balance);

    /*
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

    info!("Remove and re-init custody");
    client.remove_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
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
        end_time: utils::get_time() + 600,
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
    let user_balance_before = client.get_token_account_balance(&wallet, "SOL")?;
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
    assert_eq!(
        user_balance_before,
        client.get_token_account_balance(&wallet, "SOL")?
    );

    // request and approve
    info!("Request a new deposit and approve");
    let fund_token_balance_before =
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
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
    assert!(fund_token_balance > fund_token_balance_before);
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
        utils::get_token_balance(&client, &wd_custody_token_address)
    );
    assert_eq!(
        client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount,
        utils::get_token_balance(&client, &wd_fees_custody_token_address)
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
        end_time: utils::get_time() + 600,
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
        utils::get_token_balance(&client, &wd_custody_token_address)
    );
    assert_eq!(
        (client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2,
        utils::get_token_balance(&client, &wd_fees_custody_token_address)
    );

    // set withdrawal schedule
    info!("Set withdrawal schedule");
    assert!(client
        .request_withdrawal_fund(&user_keypair, &fund_name, token_name, 0.1)
        .is_err());
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: true,
        limit_usd: client.get_oracle_price("SOL/USD", OracleType::Pyth, 0, 0.0)? * 0.1,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&admin_keypair, &fund_name, &schedule)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.withdrawal_schedule, schedule);

    // request withdrawal
    info!("Request withdrawal over the limit");
    let fund_token_balance_after_deposit =
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    info!("Fund token balance: {}", fund_token_balance_after_deposit);
    assert!(client
        .request_withdrawal_fund(
            &user_keypair,
            &fund_name,
            token_name,
            fund_token_balance_after_deposit
        )
        .is_err());
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: true,
        limit_usd: client.get_oracle_price("SOL/USD", OracleType::Pyth, 0, 0.0)? * 0.2,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&admin_keypair, &fund_name, &schedule)?;
    info!("Request withdrawal");
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 100.0)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(
        user_info.withdrawal_request.amount,
        client.ui_amount_to_tokens_with_decimals(100.0, 6)
    );
    assert!(user_info.withdrawal_request.time > 0);
    assert!(user_info.deny_reason.is_empty());
    assert_eq!(
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?,
        fund_token_balance_after_deposit
    );

    // cancel withdrawal
    info!("Cancel withdrawal");
    client.cancel_withdrawal_fund(&user_keypair, &fund_name, token_name)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(user_info.withdrawal_request.amount, 0);
    assert_eq!(user_info.withdrawal_request.time, 0);
    assert!(user_info.deny_reason.is_empty());

    // request and deny
    info!("Request a new withdrawal and deny");
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 111.0)?;
    client.deny_withdrawal_fund(
        &admin_keypair,
        &wallet,
        &fund_name,
        token_name,
        "not allowed",
    )?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(user_info.withdrawal_request.amount, 0);
    assert_eq!(user_info.withdrawal_request.time, 0);
    assert_eq!(user_info.deny_reason, str_to_as64("not allowed")?);
    assert_eq!(
        user_info.last_withdrawal.amount,
        client.ui_amount_to_tokens_with_decimals(111.0, 6)
    );
    assert!(user_info.last_withdrawal.time > 0);

    // request and approve
    info!("Request a new withdrawal and approve");
    let initial_sol_balance = client.get_token_account_balance(&wallet, "SOL")?;
    let initial_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 121.77)?;
    client.approve_withdrawal_fund(&admin_keypair, &wallet, &fund_name, token_name, 100.0)?;
    let user_info = client.get_fund_user_info(&wallet, &fund_name, token_name)?;
    assert_eq!(user_info.withdrawal_request.amount, 0);
    assert_eq!(user_info.withdrawal_request.time, 0);
    assert!(user_info.deny_reason.is_empty());
    let withdrew_amount = client.ui_amount_to_tokens_with_decimals(100.0 - 100.0 * 0.01, 6);
    assert_eq!(user_info.last_withdrawal.amount, withdrew_amount);
    assert!(user_info.last_withdrawal.time > 0);
    let fund_token_balance3 =
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    assert!(fund_token_balance3 > 0.0 && fund_token_balance3 < fund_token_balance_after_deposit);
    assert!(client.get_token_account_balance(&wallet, "SOL")? - initial_sol_balance > 0.09);
    let new_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    assert!(
        (initial_custody_balance as f64
            - new_custody_balance as f64
            - client.ui_amount_to_tokens(0.1, "SOL")? as f64)
            .abs()
            < 100000.0
    );
    assert!(
        (((client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2
            + client.ui_amount_to_tokens(0.1 * 0.01, "SOL")?) as f64
            - utils::get_token_balance(&client, &wd_fees_custody_token_address) as f64)
            .abs()
            < 10000.0
    );

    // turn off approval requirement
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: false,
        limit_usd: client.get_oracle_price("SOL/USD", OracleType::Pyth, 0, 0.0)? * 1.5,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&admin_keypair, &fund_name, &schedule)?;

    // request instant withdrawal
    info!("Request instant withdrawal");
    let initial_sol_balance = client.get_token_account_balance(&wallet2, "SOL")?;
    let initial_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    client.request_withdrawal_fund(&user_keypair2, &fund_name, token_name, 100.0)?;
    let user_info = client.get_fund_user_info(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_info.withdrawal_request.amount, 0);
    assert_eq!(user_info.withdrawal_request.time, 0);
    assert!(user_info.deny_reason.is_empty());
    assert!(user_info.last_withdrawal.amount > 0);
    assert!(user_info.last_withdrawal.time > 0);
    let fund_token_balance4 =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert!(fund_token_balance4 > 0.0 && fund_token_balance4 < fund_token_balance2);
    // some tolerence needed due to potential SOL/USD price change
    assert!((fund_token_balance4 - fund_token_balance3).abs() / fund_token_balance3 < 0.01);
    assert!(client.get_token_account_balance(&wallet2, "SOL")? - initial_sol_balance > 0.09);
    let new_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    assert!(
        (initial_custody_balance as f64
            - new_custody_balance as f64
            - client.ui_amount_to_tokens(0.1, "SOL")? as f64)
            .abs()
            < 100000.0
    );
    assert!(
        (((client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2
            + client.ui_amount_to_tokens(0.1 * 0.01, "SOL")? * 2) as f64
            - utils::get_token_balance(&client, &wd_fees_custody_token_address) as f64)
            .abs()
            < 10000.0
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
    let trading_custody_token_address =
        client.get_fund_custody_token_account(&fund_name, token_name, FundCustodyType::Trading)?;
    let wd_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    let trading_custody_balance = utils::get_token_balance(&client, &trading_custody_token_address);
    assert_eq!(trading_custody_balance, 0);
    client.lock_assets_fund(&admin_keypair, &fund_name, token_name, 0.0)?;
    assert_eq!(
        0,
        utils::get_token_balance(&client, &wd_custody_token_address)
    );
    assert_eq!(
        wd_custody_balance,
        utils::get_token_balance(&client, &trading_custody_token_address)
    );

    // release funds into w/d custody
    info!("Release funds into w/d custody");
    client.unlock_assets_fund(&admin_keypair, &fund_name, token_name, 0.0)?;
    assert_eq!(
        0,
        utils::get_token_balance(&client, &trading_custody_token_address)
    );
    assert_eq!(
        wd_custody_balance,
        utils::get_token_balance(&client, &wd_custody_token_address)
    );*/

    Ok(())
}
