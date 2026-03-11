//! Loan data struct is used to temporarily store loan data in an account 
//! before the loan is repaid.
use pinocchio::{ AccountView, Address, error::ProgramError };

#[repr(C, packed)]
pub struct LoanData {
    pub protocol_token_account: [u8; 32],
    pub balance: u64,
}

// Read token amount from an account
pub fn get_token_amount(data: &[u8], account: &AccountView) -> Result<u64, ProgramError> {
    if !account.owned_by(&pinocchio_token::ID) {
        return Err(ProgramError::InvalidAccountOwner.into());
    }
    if account.data_len().ne(&pinocchio_token::state::TokenAccount::LEN) {
        return Err(ProgramError::InvalidAccountData.into());
    }
    if data.len() != pinocchio_token::state::TokenAccount::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(u64::from_le_bytes(data[64..72].try_into().unwrap()))
}

#[inline(always)]
pub fn pubkey_eq(a: &Address, b: &Address) -> bool {
    // Direct slice comparison is highle optimized in Solana's BPF.
    a.as_ref() == b.as_ref()
}
