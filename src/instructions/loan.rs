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
use crate::instructions::helpers::{pubkey_eq, LoanData, get_token_amount};
use crate::instructions::repay::Repay;


pub struct LoanAccounts<'a> {
    // User requesting the flash loan. Must be a signer
    pub borrower: &'a AccountView,
    // PDA that owns the protocol's liquidity pool for a specific fee
    pub protocol: &'a AccountView,
    // "Scratch" account used to save the protocol_token_account and final balance it 
    // needs to have. Must be mutable.
    pub loan: &'a AccountView,
    pub instruction_sysvar: &'a AccountView,
    // Token program. Must be executable
    pub token_accounts: &'a [AccountView],
}

impl<'a> TryFrom<&'a [AccountView]> for LoanAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        // Here, token accounts come last because they are variable length list.
        // token_program and system program are passed by the client when building the transaction
        let [borrower, protocol, loan, instruction_sysvar, _token_program, _system_program, token_accounts @ ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        // Check if this is the right sysvar account
        if !pubkey_eq(instruction_sysvar.address(), &INSTRUCTIONS_ID) {
            return Err(ProgramError::UnsupportedSysvar);
        }
        // Verify that the number of token accounts is valid
        // They are entered in pairs i.e protocol_vault_1 and user_account_1 etc
        if (token_accounts.len() % 2).ne(&0) || token_accounts.len().eq(&0) {
            return Err(ProgramError::InvalidAccountData);
        }
        // Ensures the scratch account is empty to prevent state injection attack
        if loan.try_borrow()?.len().ne(&0) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self {
            borrower, protocol, loan, instruction_sysvar, token_accounts
        })
    }
}

pub struct LoanInstructionData<'a> {
    // Used to derive the protocol's PDA instead of using the find_program_address() function
    // to save compute units. Client precomputes and sends it.
    pub bump: [u8; 1],
    // Fee rate in basis points that the users pay for borrowing
    pub fee: u16,
    // Dynamic array of loan amounts. User can request multiple loans in one transaction
    pub amounts: &'a [u64],
}

impl<'a> TryFrom<&'a [u8]> for LoanInstructionData<'a> {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // Getting the bump
        let (bump, data) = data.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        // Get the fee
        let (fee, data) = data.split_at_checked(size_of::<u16>()).ok_or(ProgramError::InvalidInstructionData)?;
        // Verify that the data is valid and also, must divide evenly
        if data.len() % size_of::<u64>() != 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        // Get the amounts
        // This converts  &[u8] to & [u64] without copying memory. It is unsafe because rust 
        // cannot guarantee alignment and correct memory layout. But here it's safe since
        // we validated data.len() % 8 == 0. This saves memory, cu and heap allocations
        let amounts: &[u64] = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const u64,
                data.len() / size_of::<u64>()
            )
        };
        Ok(Self {
            bump: [*bump],
            fee: u16::from_le_bytes(fee.try_into().map_err(|_| ProgramError::InvalidInstructionData)?),
            amounts,
        })
    }
}

pub struct Loan<'a> {
    pub accounts: LoanAccounts<'a>,
    pub instruction_data: LoanInstructionData<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Loan<'a> {
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = LoanAccounts::try_from(accounts)?;
        let instruction_data = LoanInstructionData::try_from(data)?;
        // Verify that the number of amounts matches the number of token accounts
        // Number of tokens is half the number of token accounts.
        if instruction_data.amounts.len() != accounts.token_accounts.len() / 2 {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self {
            accounts, instruction_data
        })
    }
}

impl<'a> Loan<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;
    pub fn process(&mut self) -> ProgramResult {
        log!("Running the loan instruction");
        // Get the fee
        let fee_bytes = self.instruction_data.fee.to_le_bytes();
        let (expected_loan_pda, loan_pda_bump) = Address::find_program_address(
                &[b"loan", self.accounts.protocol.address().as_ref()], &crate::ID.into()
            );
        let loan_bump = [loan_pda_bump];
        // Get the signer seeds
        let signer_seeds = [
            Seed::from("loan".as_bytes()),
            Seed::from(self.accounts.protocol.address().as_ref()),
            Seed::from(&loan_bump),
        ];
        let signer_seeds = [Signer::from(&signer_seeds)];
        // Open the LoanData account and create a mutable slice to push the Loan struct to it
        let size = size_of::<LoanData>() * self.instruction_data.amounts.len();
        let lamports = Rent::get()?.minimum_balance(size);
        log!("Creating account");
        CreateAccount {
            from: self.accounts.borrower,
            to: self.accounts.loan,
            lamports,
            space: size as u64,
            owner: &ID,
        }.invoke_signed(&signer_seeds)?;
        log!("Account created. About to create loan entries");
        // Mutable slice from the loan account's data which we populate as we process the loans and
        // their corresponding transfer.
        // Here we have the structure [u8, u8, u8, u8, etc..]
        let mut loan_data = self.accounts.loan.try_borrow_mut()?;
        log!("Assigning loan entries");
        // results into the structure [LoanData, LoanData, LoanData, etc..]
        let loan_entries = unsafe {
            core::slice::from_raw_parts_mut(
                loan_data.as_mut_ptr() as *mut LoanData,
                self.instruction_data.amounts.len()
            )
        };
        log!("Loan entries created. Introspecting the repay instruction");

        // Introspecting the Repay instruction 
        let instruction_sysvar = unsafe {
            Instructions::new_unchecked(self.accounts.instruction_sysvar.try_borrow()?)
        };
        let num_instructions = instruction_sysvar.num_instructions();
        log!("Loading the last instruction from sysvar");
        let instruction = instruction_sysvar.load_instruction_at(num_instructions as usize - 1)?;
        log!("Checking the last instruction ID");
        if instruction.get_program_id().to_bytes() != crate::ID {
            return Err(ProgramError::InvalidInstructionData);
        }
        log!("Checking the last instruction discriminator");
        if unsafe { *(instruction.get_instruction_data().as_ptr()) } != *Repay::DISCRIMINATOR {
            return Err(ProgramError::InvalidInstructionData);
        }
        // Verifies the repay instruction references the same loan account.
        // Account at index 1 is expected to be the loan account it is compared to the actual
        // loan account passed to the current instruction
        log!("Checking the last instruction references the same loan instruction");
        let repay_acc = unsafe {
            instruction.get_instruction_account_at_unchecked(1)
        };
        if repay_acc.key != *self.accounts.loan.address() {
            return Err(ProgramError::InvalidInstructionData);
        }

        log!("Begin processing transfers");
        // Processing the transfers
        for (i, amount) in self.instruction_data.amounts.iter().enumerate() {
            let protocol_token_account = &self.accounts.token_accounts[i * 2];
            let borrower_token_account = &self.accounts.token_accounts[i * 2 + 1];
            // Get the balance of the protocol's token account plus fee that remains after the loan
            // is repaid back. That is basically initial pool value (before loan) plus fee.
            log!("Getting the protocol token amount");
            let balance = get_token_amount(&protocol_token_account.try_borrow()?, &protocol_token_account)?;
            // Flash loan fee calculation typically uses basis points.
            // fee_amount = amount * fee / 10_000
            log!("Computing protocol token amount with fee");
            let balance_with_fee = balance.checked_add(
                amount.checked_mul(self.instruction_data.fee as u64)
                .and_then(|x| x.checked_div(10_000))
                .ok_or(ProgramError::InvalidInstructionData)?
            ).ok_or(ProgramError::InvalidInstructionData)?;
            // Push the loan struct into the loan account.
            log!("Creating the loan entries");
            loan_entries[i] = LoanData {
                protocol_token_account: protocol_token_account.address().to_bytes(),
                balance: balance_with_fee,
            };
            log!("Transfer instruction is here");
            // Transfer tokens from the protocol to the borrower
            Transfer {
                from: protocol_token_account,
                to: borrower_token_account,
                authority: self.accounts.protocol,
                amount: *amount,
            }.invoke_signed(&signer_seeds)?;
        }
        log!("Loan instruction ran successfully");
        Ok(())
    }
}
