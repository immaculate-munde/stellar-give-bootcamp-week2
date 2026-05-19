#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env};

#[contract]
pub struct CounterContract;

#[contractimpl]
impl CounterContract {
    pub fn increment(env: Env) -> u32 {
        let key = symbol_short!("COUNT");
        let count: u32 = env.storage().instance().get(&key).unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&key, &new_count);
        new_count
    }
}

mod test;
