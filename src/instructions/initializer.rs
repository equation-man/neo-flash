/// Initializing the protocol.
use pinocchio::{ Address, AccountView, error::ProgramError, ProgramResult };
use pinocchio::sysvars::{
    instructions::{ INSTRUCTIONS_ID },
    rent::Rent, Sysvar,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_log::log;
use solana_address;
use crate::instructions::helpers::{ ProtocolConfigState };

pub struct ProtocolInitAccounts<'a> {
    // The protocol's authority.
    pub authority: &'a AccountView,
    // The protocol's treasury
    pub treasury: &'a AccountView,
    pub instruction_sysvar: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ProtocolInitAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [authority, treasury, instruction_sysvar] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Perform a signer chech on the program's authority here.

        if !pubkey_eq(instruction_sysvar.address(), &INSTRUCTIONS_ID) {
            return Err(ProgramError::UnsupportedSysvar);
        }

        Ok(Self { authority, treasury, instruction_sysvar })
    }
}

pub struct ProtocolInitData {
    // The fee rate in basis points.
    pub fee_bps: u16,
    // The state of the protocol.
    pub paused: u8,
}

impl<'a> TryFrom<&'a [u8]> for ProtocolInitData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let (fee_bps, protocol_state) = data.split_at_checked(size_of::<u16>()).ok_or(ProgramError::InvalidInstructionData)?;

        if protocol_state != 0 || protocol_state != 1 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { fee_bps, protocol_state })
    }
}

pub struct ProtocolInitializer<'a> {
    pub accounts: ProtocolInitAccounts<'a>,
    pub instructin_data: ProtocolInitData<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ProtocolInitializer<'a> {
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = ProtocolInitAccounts::try_from(accounts)?;
        let instruction_data = ProtocolInitData::try_from(data)?;

        Ok(Self { accounts, instruction_data })
    }
}
