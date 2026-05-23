use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};

use crate::{
    error::ContractError,
    events::{Approval, Burn, Mint, Transfer},
    storage::{AllowanceKey, AllowanceValue, DataKey},
    token_trait::TokenInterface,
};

#[contract]
pub struct SibToken;

impl SibToken {
    fn check_nonnegative_amount(amount: i128) {
        if amount < 0 {
            panic!("negative amount is not allowed");
        }
    }

    fn read_balance(env: &Env, id: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id.clone()))
            .unwrap_or(0)
    }

    fn write_balance(env: &Env, id: &Address, amount: i128) {
        env.storage()
            .persistent()
            .set(&DataKey::Balance(id.clone()), &amount);
    }

    fn receive_balance(env: &Env, id: Address, amount: i128) {
        let balance = Self::read_balance(env, &id);
        Self::write_balance(env, &id, balance + amount);
    }

    fn spend_balance(env: &Env, id: Address, amount: i128) -> Result<(), ContractError> {
        let balance = Self::read_balance(env, &id);
        if balance < amount {
            return Err(ContractError::InsufficientFunds);
        }
        Self::write_balance(env, &id, balance - amount);
        Ok(())
    }

    fn read_allowance(env: &Env, from: &Address, spender: &Address) -> AllowanceValue {
        let key = DataKey::Allowance(AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        });
        if let Some(allowance) = env.storage().persistent().get::<_, AllowanceValue>(&key) {
            if allowance.live_until_ledger < env.ledger().sequence() {
                AllowanceValue {
                    amount: 0,
                    live_until_ledger: allowance.live_until_ledger,
                }
            } else {
                allowance
            }
        } else {
            AllowanceValue {
                amount: 0,
                live_until_ledger: 0,
            }
        }
    }

    fn write_allowance(
        env: &Env,
        from: Address,
        spender: Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        let key = DataKey::Allowance(AllowanceKey {
            from: from.clone(),
            spender: spender.clone(),
        });
        env.storage().persistent().set(
            &key,
            &AllowanceValue {
                amount,
                live_until_ledger,
            },
        );
    }

    fn spend_allowance(
        env: &Env,
        from: Address,
        spender: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        let allowance = Self::read_allowance(env, &from, &spender);
        if allowance.amount < amount {
            return Err(ContractError::InsufficientAllowance);
        }
        Self::write_allowance(
            env,
            from,
            spender,
            allowance.amount - amount,
            allowance.live_until_ledger,
        );
        Ok(())
    }
}

#[contractimpl]
impl SibToken {
    /// Deploy-time initialization. Mints `initial_supply` to `admin` when > 0.
    pub fn __constructor(env: Env, admin: Address, initial_supply: i128) {
        Self::check_nonnegative_amount(initial_supply);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        if initial_supply > 0 {
            Self::receive_balance(&env, admin.clone(), initial_supply);
            Mint {
                to: admin,
                amount: initial_supply,
            }
            .publish(&env);
        }
    }

    /// Mint new tokens to `to`. Only the stored admin may call this.
    pub fn mint(env: Env, to: Address, amount: i128) {
        Self::check_nonnegative_amount(amount);
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("admin not set");
        admin.require_auth();
        Self::receive_balance(&env, to.clone(), amount);
        Mint { to, amount }.publish(&env);
    }
}

#[contractimpl(contracttrait)]
impl TokenInterface for SibToken {
    fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        Self::read_allowance(&env, &from, &spender).amount
    }

    fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        from.require_auth();
        Self::check_nonnegative_amount(amount);

        if amount > 0 && live_until_ledger < env.ledger().sequence() {
            panic!("expiration_ledger is less than current ledger sequence");
        }

        let from_balance = Self::read_balance(&env, &from);
        if from_balance < amount {
            panic!("insufficient balance");
        }

        Self::write_allowance(
            &env,
            from.clone(),
            spender.clone(),
            amount,
            live_until_ledger,
        );

        Approval {
            from,
            spender,
            amount,
            live_until_ledger,
        }
        .publish(&env);
    }

    fn balance(env: Env, id: Address) -> i128 {
        Self::read_balance(&env, &id)
    }

    fn transfer(env: Env, from: Address, to_muxed: MuxedAddress, amount: i128) {
        from.require_auth();
        Self::check_nonnegative_amount(amount);

        let to = to_muxed.address();
        Self::spend_balance(&env, from.clone(), amount)
            .unwrap_or_else(|_| panic!("insufficient balance"));
        Self::receive_balance(&env, to.clone(), amount);

        Transfer {
            from,
            to,
            amount,
        }
        .publish(&env);
    }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        Self::check_nonnegative_amount(amount);

        Self::spend_allowance(&env, from.clone(), spender, amount)
            .unwrap_or_else(|_| panic!("insufficient allowance"));
        Self::spend_balance(&env, from.clone(), amount)
            .unwrap_or_else(|_| panic!("insufficient balance"));
        Self::receive_balance(&env, to.clone(), amount);

        Transfer {
            from,
            to,
            amount,
        }
        .publish(&env);
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        Self::check_nonnegative_amount(amount);

        Self::spend_balance(&env, from.clone(), amount)
            .unwrap_or_else(|_| panic!("insufficient balance"));

        Burn { from, amount }.publish(&env);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        Self::check_nonnegative_amount(amount);

        Self::spend_allowance(&env, from.clone(), spender, amount)
            .unwrap_or_else(|_| panic!("insufficient allowance"));
        Self::spend_balance(&env, from.clone(), amount)
            .unwrap_or_else(|_| panic!("insufficient balance"));

        Burn { from, amount }.publish(&env);
    }

    fn decimals(_env: Env) -> u32 {
        18
    }

    fn name(env: Env) -> String {
        String::from_str(&env, "SibToken")
    }

    fn symbol(env: Env) -> String {
        String::from_str(&env, "SIB")
    }
}
