//! It checks if all the balances have been correctly paid using the loan account and close the
//! loan account since it is nolonger needed.
//! Here, no instruction data is needed, we will be using the balance field in the loan account
//! to verify if the loan has been repaid
//! Repayment doesn't happen in this instruction. The borrower can choose to repay the
//! token account in another instruction, such as when performing a swap or executing a series of
//! CPIs from the arbitrage
use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError};
use pinocchio::sysvars::{
    instructions::Instructions
};
use pinocchio_token::instructions::Transfer;
use crate::instructions::helpers::{
    ProtocolConfigState, get_token_amount,
};
use crate::instructions::loan::Loan;
use pinocchio_log::log;

pub struct RepayAccounts<'a> {
    // Who requsted the loan
    pub borrower: &'a AccountView,
    // Where the borrowed tokens went to.
    pub borrower_token_account: &'a AccountView,
    // The protocol configuration.
    pub config: &'a AccountView,
    // Who offered the loan or temporary liquidity
    pub liquidity_vault: &'a AccountView,
    pub instruction_sysvar: &'a AccountView,
    // Since we are performing token transfer.
    pub token_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for RepayAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [borrower, borrower_token_account, config, liquidity_vault, instruction_sysvar, token_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        Ok(Self {
            borrower, borrower_token_account, config,
            liquidity_vault, instruction_sysvar, token_program
        })
    }
}

pub struct Repay<'a> {
    pub accounts: RepayAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountView]> for Repay<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let accounts = RepayAccounts::try_from(accounts)?;
        Ok(Self { accounts })
    }
}

impl<'a> Repay<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;
    pub fn process(&mut self) -> ProgramResult {
        // Config PDA.
        let config = ProtocolConfigState::load(self.accounts.config)?;
        if config.protocol_state == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Introspecting the Loan instruction to retrieve the loan state for validation.
        // We had already checked the loan repayment accound in the borrow instruction so we skip
        // it here.
        let instruction_sysvar = unsafe {
            Instructions::new_unchecked(self.accounts.instruction_sysvar.try_borrow()?)
        };
        let borrow_ix = instruction_sysvar.load_instruction_at(0)?;
        if borrow_ix.get_program_id().to_bytes() != crate::ID {
            return Err(ProgramError::InvalidInstructionData);
        }
        if unsafe { *(borrow_ix.get_instruction_data().as_ptr()) } != *Loan::DISCRIMINATOR {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Get the balance of the protocol's token account plus fee that remains after the loan
        // is repaid back. That is basically initial pool value (before loan) plus fee.
        //let initial_balance = get_token_amount(&self.accounts.liquidity_vault)?;
        
        // Extracting the borrow instruction data slice. This points directly where transaction
        // data lives
        let borrow_ix_data = borrow_ix.get_instruction_data();
        let borrowed_amount = unsafe {
            // Skip the discriminator, and take the transaction data at offset 1
            let amount_ptr = borrow_ix_data.as_ptr().add(1) as *const u64;
            // read_unaligned will safely extract the 8 bytes anyway after moving the pointer by a
            // single byte causing "mis-alignment".
            // from_le() Will convert the raw little endian byte back to standart native u64.
            u64::from_le(amount_ptr.read_unaligned())
        };

        // Flash loan fee calculation typically uses basis points.
        // fee_amount = amount * fee / 10_000
        let fee_amount = borrowed_amount.checked_mul(config.fee_bps as u64)
            .and_then(|x| x.checked_div(10_000))
            .ok_or(ProgramError::InvalidInstructionData)?;
        // We repay the amount borrowed plus fee.
        let repay_amount = borrowed_amount.checked_add(fee_amount).ok_or(ProgramError::InvalidInstructionData)?;

        // Final balance of the liquidity pool should be greater than initial balance.
        if repay_amount < borrowed_amount {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Repaying back the loan
        Transfer {
            from: self.accounts.borrower_token_account,
            to: self.accounts.liquidity_vault,
            authority: self.accounts.borrower,
            amount: repay_amount,
        }.invoke()?; // Regular invoke since borrower signed the transaction
        Ok(())
    }
}
