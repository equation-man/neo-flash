//! This file contain the protocol's utilities such as necessary PDAs.
//! Loan data struct is used to temporarily store loan data in an account 
//! before the loan is repaid.
use pinocchio::{ AccountView, Address, error::ProgramError };

// Read token amount from an account
pub fn get_token_amount(account: &AccountView) -> Result<u64, ProgramError> {
    // Verify ownership against the legacy token program ID
    if !account.owned_by(&pinocchio_token::ID) {
        return Err(ProgramError::InvalidAccountOwner.into());
    }
    // Extract underlying data slice
    let data = account.try_borrow()?;

    // Legacy token accounts are static and should match exactly the base length (165 bytes).
    if data.len() != pinocchio_token::state::TokenAccount::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    // Safely slice and parse the amount.
    Ok(u64::from_le_bytes(data[64..72].try_into().unwrap()))
}

#[inline(always)]
pub fn pubkey_eq(a: &Address, b: &Address) -> bool {
    // Direct slice comparison is highle optimized in Solana's BPF.
    a.as_ref() == b.as_ref()
}

// This is the protocol's configuration state.
pub struct ProtocolConfigState {
    // Percentage fee that the protocol takes on every transaction.
    pub fee_bps: u16,
    // The protocol's update authority.
    pub authority: Address,
    // The treasury or wallet where the fees are collected to.
    pub treasury: Address,
    // The protocol state.
    pub protocol_state: u8,
}

impl ProtocolConfigState {
    // ================== READING DATA =======================
    #[inline(always)]
    pub fn load(account_info: &AccountView) -> Result<&Self, ProgramError> {
        // Load the config data.
        if account_info.data_len() != size_of::<ProtocolConfigState>() {
            return Err(ProgramError::InvalidAccountData.into());
        }

        // Check if this config account is really for this program.
        unsafe {
            if account_info.owner().ne(&Address::from(crate::ID)) {
                return Err(ProgramError::InvalidAccountData.into());
            }
        }

        // try_borrow() will safely fail if another part of the memory is accessing this data.
        let data = account_info.try_borrow()?;
        let res = unsafe {
            &*(data.as_ptr() as *const ProtocolConfigState)
        };
        Ok(res)
    }

    #[inline(always)]
    pub fn load_unchecked(account_info: &AccountView) -> Result<&Self, ProgramError> {
        if account_info.data_len() != size_of::<ProtocolConfigState>() {
            return Err(ProgramError::InvalidAccountData.into());
        }

        unsafe {
            if account_info.owner() != &Address::from(crate::ID) {
                return Err(ProgramError::InvalidAccountData.into());
            }
        }

        Ok(unsafe {
            Self::from_bytes_unchecked(
                account_info.borrow_unchecked(),
            )
        })
    }

    // Return config from given bytes.
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        &*(bytes.as_ptr() as *const ProtocolConfigState)
    }

    // Getter methods for safe field access.
    #[inline(always)]
    pub fn fee_bps(&self) -> u16 { self.fee_bps }
    #[inline(always)]
    pub fn authority(&self) -> &Address { &self.authority }
    #[inline(always)]
    pub fn treasury(&self) -> &Address { &self.treasury }
    #[inline(always)]
    pub fn protocol_state(&self) -> u8 { self.protocol_state }


    // =================== WRITING DATA ======================
    // Returnin mutable config from the given bytes.
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked_mut(bytes: &mut [u8]) -> &mut Self {
        &mut *(bytes.as_mut_ptr() as *mut ProtocolConfigState)
    }

    #[inline(always)]
    pub fn load_mut(account_info: &AccountView) -> Result<&mut Self, ProgramError> {
        if account_info.data_len() != size_of::<ProtocolConfigState>() {
            return Err(ProgramError::InvalidAccountData.into());
        }
        unsafe {
            if account_info.owner().ne(&Address::from(crate::ID)) {
                return Err(ProgramError::InvalidAccountData.into());
            }
        }
        let mut data = account_info.try_borrow_mut()?;
        // Converting RefMut<[u8]> to &mut [u8]
        let unsafe_data = unsafe {
            &mut *(data.as_mut_ptr() as *mut ProtocolConfigState)
        };
        Ok(unsafe_data)
    }

    #[inline(always)]
    pub fn set_fee(&mut self, fee: u16) -> Result<(), ProgramError> {
        if fee.ge(&10_000) {
            return Err(ProgramError::InvalidAccountData.into());
        }
        self.fee_bps = fee;
        Ok(())
    }

    #[inline(always)]
    pub fn set_authority(&mut self, authority: [u8; 32]) -> Result<(), ProgramError> {
        self.authority = authority.into();
        Ok(())
    }

    #[inline(always)]
    pub fn set_treasury(&mut self, treasury: [u8; 32]) -> Result<(), ProgramError> {
        self.treasury = treasury.into();
        Ok(())
    }

    #[inline(always)]
    pub fn set_protocol_state(&mut self, state: u8) -> Result<(), ProgramError> {
        self.protocol_state = state;
        Ok(())
    }

    #[inline(always)]
    pub fn set_inner(&mut self, fee_bps: u16, state: u8, authority: [u8; 32], treasury: [u8; 32]) -> Result<(), ProgramError> {
        self.set_fee(fee_bps);
        self.set_authority(authority);
        self.set_treasury(treasury);
        self.set_protocol_state(state);
        Ok(())
    }
}
