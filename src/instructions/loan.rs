//! Initiating the loan
use pinocchio::{ Address, AccountView, error::ProgramError, ProgramResult };
use pinocchio::sysvars::{
    instructions::{ Instructions, INSTRUCTIONS_ID },
    rent::Rent, Sysvar, 
};
use pinocchio::cpi::{Signer, Seed};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::{ Transfer };
use pinocchio_token::ID;
use pinocchio_log::log;
use solana_address;
use crate::instructions::helpers::{ProtocolConfigState, pubkey_eq, get_token_amount};
use crate::instructions::repay::Repay;


pub struct LoanAccounts<'a> {
    // User requesting the flash loan. Must be a signer
    pub borrower: &'a AccountView,
    // Wher the borrowed tokens go.
    pub borrower_token_account: &'a AccountView,
    // The configuration account.
    pub config: &'a AccountView,
    // The liquidity_vault or source of liquidity.
    pub liquidity_vault: &'a AccountView,
    // The liquidity vault PDA for signing loan transfers.
    pub liquidity_vault_pda: &'a AccountView,
    pub instruction_sysvar: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for LoanAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        // Here, token accounts come last because they are variable length list.
        // token_program and system program are passed by the client when building the transaction
        let [borrower, borrower_token_account, config, liquidity_vault, liquidity_vault_pda, instruction_sysvar, _token_program, _system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        // Check if this is the right sysvar account
        if !pubkey_eq(instruction_sysvar.address(), &INSTRUCTIONS_ID) {
            return Err(ProgramError::UnsupportedSysvar);
        }

        Ok(Self {
            borrower, borrower_token_account, config, liquidity_vault, liquidity_vault_pda, instruction_sysvar
        })
    }
}

pub struct LoanInstructionData {
    // Loan amount the user is taking 
    pub amounts: u64,
}

impl<'a> TryFrom<&'a [u8]> for LoanInstructionData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // Getting the bump
        let (amnts, data) = data.split_at_checked(size_of::<u64>()).ok_or(ProgramError::InvalidInstructionData)?;
        // Amounts will be byte stream i.e [u8]
        let amounts = u64::from_le_bytes(
            amnts.try_into().map_err(|_| ProgramError::InvalidInstructionData)?
        );
        if amounts == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self { amounts })
    }
}

pub struct Loan<'a> {
    pub accounts: LoanAccounts<'a>,
    pub instruction_data: LoanInstructionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Loan<'a> {
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = LoanAccounts::try_from(accounts)?;
        let instruction_data = LoanInstructionData::try_from(data)?;
        Ok(Self {
            accounts, instruction_data
        })
    }
}

impl<'a> Loan<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;
    pub fn process(&mut self) -> ProgramResult {
        // Load the config pda.
        let config = ProtocolConfigState::load(self.accounts.config)?;
        if config.protocol_state == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Introspecting the Repay instruction 
        let instruction_sysvar = unsafe {
            Instructions::new_unchecked(self.accounts.instruction_sysvar.try_borrow()?)
        };
        let num_instructions = instruction_sysvar.num_instructions();
        // Loading the repay instruction.
        let instruction = instruction_sysvar.load_instruction_at(num_instructions as usize - 1)?;
        if instruction.get_program_id().to_bytes() != crate::ID {
            return Err(ProgramError::InvalidInstructionData);
        }
        if unsafe { *(instruction.get_instruction_data().as_ptr()) } != *Repay::DISCRIMINATOR {
            return Err(ProgramError::InvalidInstructionData);
        }
        // Account we are repay back the loan to.
        let repay_acc = unsafe {
            instruction.get_instruction_account_at_unchecked(2)
        };
        if repay_acc.key != *self.accounts.liquidity_vault.address() {
            return Err(ProgramError::InvalidInstructionData);
        }


        // This is a self funded flash loan which provides its own liquidity.
        let (liquidity_pda, liquidity_bump) = Address::find_program_address(
                &[b"liquidity_pda"], &crate::ID.into()
            );
        let liquidity_decons_bump = [liquidity_bump];
        if liquidity_pda != *self.accounts.liquidity_vault_pda.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        // Get the signer seeds
        let protocol_liquidity_seeds = [
            Seed::from("liquidity_pda".as_bytes()),
            Seed::from(&liquidity_decons_bump),
        ];
        let protocol_signer_seeds = [Signer::from(&protocol_liquidity_seeds)];
        log!("Transfer instruction is here");
        // Transfer tokens from the protocol to the borrower
        Transfer {
            from: self.accounts.liquidity_vault,
            to: self.accounts.borrower_token_account,
            authority: self.accounts.liquidity_vault_pda,
            amount: self.instruction_data.amounts,
        }.invoke_signed(&protocol_signer_seeds)?;
        Ok(())
    }
}
