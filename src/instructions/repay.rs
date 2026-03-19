//! It checks if all the balances have been correctly paid using the loan account and close the
//! loan account since it is nolonger needed.
//! Here, no instruction data is needed, we will be using the balance field in the loan account
//! to verify if the loan has been repaid
//! Repayment doesn't happen in this instruction. The borrower can choose to repay the
//! token account in another instruction, such as when performing a swap or executing a series of
//! CPIs from the arbitrage
use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError};
use crate::instructions::helpers::{
    LoanData, get_token_amount,
};
use pinocchio_log::log;

pub struct RepayAccounts<'a> {
    // Who requsted the loan
    pub borrower: &'a AccountView,
    // Stores protocol_token_account and the final balance
    pub loan: &'a AccountView,
    // Protocol token accounts associated with the borrower's loan
    pub token_accounts: &'a [AccountView],
}

impl<'a> TryFrom<&'a [AccountView]> for RepayAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [borrower, loan, token_accounts @ ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        Ok(Self {
            borrower, loan, token_accounts
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
    pub const DISCRIMINATOR: &'a u8 = &1;
    pub fn process(&mut self) -> ProgramResult {
        let loan_data = self.accounts.loan.try_borrow()?;
        let loan_num = loan_data.len() / size_of::<LoanData>();
        if loan_num.ne(&self.accounts.token_accounts.len()) {
            return Err(ProgramError::InvalidAccountData);
        }
        // Process each pair of token accounts (protocol, borrower) with corresponding amounts
        for i in 0..loan_num {
            // Validating that the protocol_ata is the same as the one in the loan account.
            let protocol_token_account = &self.accounts.token_accounts[i];
            // Pointer to the offset where the loan LoanData starts
            if unsafe { *(loan_data.as_ptr().add(i * size_of::<LoanData>()) as *const [u8; 32]) } != protocol_token_account.address().to_bytes() {
                return Err(ProgramError::InvalidAccountData);
            }
            // Check if the loan is already repaid
            let balance = get_token_amount(&protocol_token_account.try_borrow()?, &protocol_token_account)?;
            // Checking the second field of loan data or the final balance.
            let loan_balance = unsafe { *(loan_data.as_ptr().add(i * size_of::<LoanData>() + size_of::<[u8; 32]>()) as *const u64) };
            // Checking if the final balance is greater than or equal to the original amount.
            if balance < loan_balance {
                return Err(ProgramError::InvalidAccountData);
            }
        }
        // Reclaim the loan account and its rent.
        drop(loan_data);
        // Closing the loan account an giving back the lamports to the borrower.
        // SAFELY tranfer lamports back to borrower
        let loan_lamports = self.accounts.loan.lamports();
        self.accounts.loan.set_lamports(0);
        self.accounts.borrower.set_lamports(
            self.accounts.borrower.lamports()
            .checked_add(loan_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?
        );
        self.accounts.loan.set_lamports(0);
        // Close the loan account, this zeroes out data and marks it closed.
        unsafe {
            self.accounts.loan.close_unchecked();
        }
        log!("The repay instruction is successful");
        Ok(())
    }
}
