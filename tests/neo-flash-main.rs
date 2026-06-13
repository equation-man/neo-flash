//! Main test file to test for flash loan transactions.
#![allow(warnings)]
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    signature::{ Signer },
};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
mod setup_helpers;
use crate::setup_helpers::{
    NeoFlashConfigContext, initialize_protocol,
    init_test_env, test_borrow_ix,
};

#[test]
fn test_flash_loan() {
    // Initializing the protocl.
    let mut ctx_init = initialize_protocol();
    // Setting up the loan environment, e.g liquidity pools.
    let mut test_env = init_test_env(&mut ctx_init.svm, ctx_init.program_id);
    // Executing the flash loan.
    let test_borrow = test_borrow_ix(ctx_init, test_env);
}
