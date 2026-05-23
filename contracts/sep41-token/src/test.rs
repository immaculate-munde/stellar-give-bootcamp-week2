#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, MuxedAddress, String};

use crate::our_token::{SibToken, SibTokenClient};

struct SetUpResult<'a> {
    env: Env,
    client: SibTokenClient<'a>,
    admin: Address,
    sender: Address,
    receiver: Address,
}

fn setup<'a>(initial_supply: i128) -> SetUpResult<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(SibToken, (admin.clone(), initial_supply));
    let client = SibTokenClient::new(&env, &contract_id);

    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);

    SetUpResult {
        env,
        client,
        admin,
        sender,
        receiver,
    }
}

#[test]
fn test_name() {
    let setup_result = setup(0);
    let name = setup_result.client.name();
    let token_name = String::from_str(&setup_result.env, "SibToken");
    assert_eq!(name, token_name);
}

#[test]
fn test_symbol() {
    let setup_result = setup(0);
    let name = setup_result.client.symbol();
    let token_name = String::from_str(&setup_result.env, "SIB");
    let not_token_name = String::from_str(&setup_result.env, "Sib");
    assert_eq!(name, token_name);
    assert_ne!(name, not_token_name);
}

#[test]
fn test_decimal() {
    let setup_result = setup(0);
    assert_eq!(setup_result.client.decimals(), 18);
}

#[test]
fn test_constructor_mint() {
    let setup_result = setup(5000);
    assert_eq!(setup_result.client.balance(&setup_result.admin), 5000);
}

#[test]
fn test_transfer() {
    let setup_result = setup(0);
    let client = &setup_result.client;

    client.mint(&setup_result.sender, &1000);
    assert_eq!(client.balance(&setup_result.sender), 1000);

    client.transfer(
        &setup_result.sender,
        &MuxedAddress::from(setup_result.receiver.clone()),
        &600,
    );
    assert_eq!(client.balance(&setup_result.sender), 400);
    assert_eq!(client.balance(&setup_result.receiver), 600);
}

#[test]
fn test_transfer_from() {
    let setup_result = setup(0);
    let client = &setup_result.client;
    let spender = Address::generate(&setup_result.env);
    let recipient = Address::generate(&setup_result.env);

    client.mint(&setup_result.sender, &1000);
    client.approve(&setup_result.sender, &spender, &500, &200);

    client.transfer_from(&spender, &setup_result.sender, &recipient, &400);
    assert_eq!(client.balance(&setup_result.sender), 600);
    assert_eq!(client.balance(&recipient), 400);
    assert_eq!(client.allowance(&setup_result.sender, &spender), 100);
}

#[test]
fn test_burn() {
    let setup_result = setup(0);
    let client = &setup_result.client;
    let spender = Address::generate(&setup_result.env);

    client.mint(&setup_result.sender, &1000);
    client.approve(&setup_result.sender, &spender, &500, &200);

    client.burn_from(&spender, &setup_result.sender, &500);
    assert_eq!(client.allowance(&setup_result.sender, &spender), 0);
    assert_eq!(client.balance(&setup_result.sender), 500);

    client.burn(&setup_result.sender, &500);
    assert_eq!(client.balance(&setup_result.sender), 0);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_insufficient_balance() {
    let setup_result = setup(0);
    let client = &setup_result.client;

    client.mint(&setup_result.sender, &1000);
    client.transfer(
        &setup_result.sender,
        &MuxedAddress::from(setup_result.receiver.clone()),
        &1001,
    );
}

#[test]
#[should_panic(expected = "insufficient allowance")]
fn transfer_from_insufficient_allowance() {
    let setup_result = setup(0);
    let client = &setup_result.client;
    let spender = Address::generate(&setup_result.env);

    client.mint(&setup_result.sender, &1000);
    client.approve(&setup_result.sender, &spender, &100, &200);
    client.transfer_from(&spender, &setup_result.sender, &setup_result.receiver, &101);
}
