//! Setting up the required state for the flash loan test.
use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{ Signer, Keypair},
    transaction::Transaction,
};
use solana_program::program_pack::Pack; // Trait to enable Mint::LEN
use spl_token::{
    state::{Mint, Account as TokenAccount},
    ID as TOKEN_PROGRAM_ID,
    instruction as token_instruction
};

pub struct NeoFlashTestContext {
    pub svm: LiteSVM,
    pub borrower: Keypair,
    pub protocol_pda: Pubkey,
    pub loan: Keypair,
    pub mint: Pubkey,
    pub protocol_token: Pubkey,
    pub borrower_token: Pubkey,
    pub program_id: Pubkey,
}

pub fn setup_neo_flash_context() -> NeoFlashTestContext {
    let program_id = solana_sdk::pubkey!("DnWWkqtWVwv5bVc4mnnvxMvZZUsuYNCpZQHGPixbqm4v");
    let bytes = include_bytes!("../target/deploy/neo_flash.so");
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, bytes);

    let borrower = Keypair::new();
    let protocol = Keypair::new();
    let loan = Keypair::new();

    // Giving borrower SOL for transactions fees
    svm.airdrop(&borrower.pubkey(), 5_000_000_000).unwrap();

    // =========== protocol PDA will own the vault containing the liquidity  ========
    let (protocol_pda, protocol_bump) = Pubkey::find_program_address(
        &[b"protocol"],
        &program_id,
    );

    // =====================Create mint: Owned by the token program====================
    let mint = Keypair::new();
    // creating the mint account state.
    let mint_state = Mint {
        mint_authority: Some(protocol_pda).into(),
        supply: 1_000_000_000,
        decimals: 6,
        is_initialized: true, // Initializing the accounts
        freeze_authority: None.into(),
    };
    // Create the account's byte buffer
    let mut mint_data = vec![0u8; Mint::LEN];
    // Serialize the mint_state into that buffer
    Mint::pack(mint_state, &mut mint_data).unwrap();
    svm.set_account(
        mint.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: mint_data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    ).unwrap();

    // ================ Setup protocol vault =================
    let protocol_token = Keypair::new();
    // Token account state
    let protocol_token_state = TokenAccount {
        mint: mint.pubkey(),
        owner: protocol_pda, // The pda owns the vault
        amount: 1_000_000_000, // Inititial liquidity
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    };
    let mut protocol_token_data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(protocol_token_state, &mut protocol_token_data).unwrap();
    svm.set_account(
        protocol_token.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: protocol_token_data,
            owner: spl_token::id(), // Standard SPL token program
            executable: false,
            rent_epoch: 0,
        },
    ).unwrap();

    // ================== Setup borrower token Account ==========================
    let borrower_token = Keypair::new();
    let borrower_state = TokenAccount {
        mint: mint.pubkey(),
        owner: borrower.pubkey(),
        amount: 1_000_000_000,
        state: spl_token::state::AccountState::Initialized,
        ..Default::default()
    };
    let mut borrower_token_data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(borrower_state, &mut borrower_token_data).unwrap();
    svm.set_account(
        borrower_token.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: borrower_token_data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        }
    ).unwrap();
    //let init_borrower_token_ix = token_instruction::initialize_account(
    //    &TOKEN_PROGRAM_ID, &borrower_token.pubkey(), &mint.pubkey(), &borrower.pubkey()
    //).unwrap();

    // Mint liquidity into protocol vault. Requires protocol (mint authority) signature
    //let mint_to_ix = token_instruction::mint_to(
    //    &TOKEN_PROGRAM_ID, &mint.pubkey(), &protocol_token.pubkey(), &protocol_pda, &[], 1_000_000_000
    //).unwrap();
    //let tx = Transaction::new_signed_with_payer(
    //    &[mint_to_ix],
    //    Some(&borrower.pubkey()), // BOrrower pays for the setup transaction
    //    &[&borrower], // Protocol must sign to authrize mint
    //    svm.latest_blockhash()
    //);
    //svm.send_transaction(tx).unwrap();

    NeoFlashTestContext {
        svm,
        borrower,
        // Authority controlling the pool
        protocol_pda,
        // Scratch PDA used to store LoanData
        loan,
        // SPL token used for the flash loan
        mint: mint.pubkey(),
        // Liquidity pool vault holding funds
        protocol_token: protocol_token.pubkey(),
        // Token account receiving the loan
        borrower_token: borrower_token.pubkey(),
        program_id,
    }
}

pub fn get_token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let acc = svm.get_account(token_account).unwrap();
    let token_acc = TokenAccount::unpack(&acc.data).unwrap();
    token_acc.amount
}
