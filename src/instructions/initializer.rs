/// Initializing the protocol.
use pinocchio::{ Address, AccountView, error::ProgramError, ProgramResult };
use pinocchio::sysvars::{
    instructions::{ INSTRUCTIONS_ID },
    rent::Rent, Sysvar,
};
use pinocchio::cpi::{Signer, Seed};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_log::log;
use solana_address;
use crate::instructions::helpers::{ ProtocolConfigState };

pub struct ProtocolInitAccounts<'a> {
    // The protocol's authority.
    pub authority: &'a AccountView,
    // The protocol's treasury
    pub treasury: &'a AccountView,
    // Configuration account to be initialized.
    pub config: &'a AccountView,
    // The system program id
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ProtocolInitAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [authority, treasury, config, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Perform a signer check on the program's authority here.
        if !authority.is_signer() {
            return Err(ProgramError::InvalidAccountData);
        }

        //if !pubkey_eq(instruction_sysvar.address(), &INSTRUCTIONS_ID) {
        //    return Err(ProgramError::UnsupportedSysvar);
        //}

        Ok(Self { authority, treasury, config, system_program})
    }
}

pub struct ProtocolInitData {
    // The fee rate in basis points.
    pub fee_bps: u16,
    // The state of the protocol.
    pub protocol_state: u8,
}

impl<'a> TryFrom<&'a [u8]> for ProtocolInitData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != 3 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let fee_bps = u16::from_le_bytes([data[0], data[1]]);
        let protocol_state = data[2];

        if protocol_state > 1 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { fee_bps, protocol_state })
    }
}

pub struct ProtocolInitializer<'a> {
    pub accounts: ProtocolInitAccounts<'a>,
    pub instruction_data: ProtocolInitData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ProtocolInitializer<'a> {
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let accounts = ProtocolInitAccounts::try_from(accounts)?;
        let instruction_data = ProtocolInitData::try_from(data)?;

        Ok(Self { accounts, instruction_data })
    }
}

impl<'a> ProtocolInitializer<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;
    pub fn process(&mut self) -> ProgramResult {
        log!("Initializing the flash loan protocol");
        // Generating the config PDA.
        let (protocol_config_pda, config_pda_bump) = Address::find_program_address(
            &[b"config", self.accounts.treasury.address().as_ref()],
            &crate::ID.into()
        );
        let config_bump = [config_pda_bump];
        // Get the signer seeds
        let signer_seeds = [
            Seed::from("config".as_bytes()),
            Seed::from(self.accounts.treasury.address().as_ref()),
            Seed::from(&config_bump),
        ];
        let signer_seeds = [Signer::from(&signer_seeds)];
        let size = size_of::<ProtocolConfigState>();
        let lamports = Rent::get()?.minimum_balance(size);
        CreateAccount {
            from: self.accounts.authority,
            to: self.accounts.config,
            lamports,
            space: size as u64,
            owner: &crate::ID.into(),
        }.invoke_signed(&signer_seeds)?;
        Ok(())
    }
}
