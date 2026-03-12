//! Our smart contract's entrypoint.
#![allow(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use pinocchio::{
    AccountView, entrypoint, error::ProgramError,
    ProgramResult, Address,
};
use pinocchio_pubkey::declare_id;
use pinocchio_log::log;

mod instructions;
use instructions::{
    loan::*,
    repay::*,
    helpers::*,
};

declare_id!("DnWWkqtWVwv5bVc4mnnvxMvZZUsuYNCpZQHGPixbqm4v");
entrypoint!(process_instruction);
fn process_instruction(_program_id: &Address, accounts: &[AccountView], instruction_data: &[u8]) -> ProgramResult {
    match instruction_data.split_first() {
        Some((Loan::DISCRIMINATOR, data)) => Loan::try_from((data, accounts))?.process(),
        Some((Repay::DISCRIMINATOR, _)) => Repay::try_from(accounts)?.process(),
        _ => Err(ProgramError::InvalidInstructionData)
    }
}
