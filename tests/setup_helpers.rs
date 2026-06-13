//! Setting up the required state for the flash loan test.
#![allow(warnings)]
use litesvm::LiteSVM;
use litesvm_token::{
    get_spl_account,
    spl_token::{native_mint::DECIMALS, state::Account as TknAccount},
    CreateAssociatedTokenAccount, CreateMint, MintTo,
};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    message::Message,
    signature::{ Signer, Keypair},
    transaction::Transaction,
};
use solana_program::program_pack::Pack; // Trait to enable Mint::LEN
use solana_program::sysvar::instructions::ID as SYSVARS_ID;
use spl_token::{
    state::{Mint, Account as TokenAccount},
    ID as TOKEN_PROGRAM_ID,
    instruction as token_instruction
};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;

// Protocol configuration context.
pub struct NeoFlashConfigContext {
    pub svm: LiteSVM,
    // The protocol's authority
    pub authority: Keypair,
    // The protocol's treasury where fee is stored
    pub treasury: Pubkey,
    pub program_id: Pubkey,
}

pub fn initialize_protocol() -> NeoFlashConfigContext {
    let program_id = solana_sdk::pubkey!("DnWWkqtWVwv5bVc4mnnvxMvZZUsuYNCpZQHGPixbqm4v");
    let bytes = include_bytes!("../target/deploy/neo_flash.so");
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, bytes);

    // Accounts needed for the instruction.
    let authority = Keypair::new();
    let treasury = Pubkey::new_unique();
    let sysvar_accnt = Pubkey::new_unique();

    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[b"config", treasury.as_ref()],
        &program_id,
    );

    // Giving authority SOL for transactions fees
    svm.airdrop(&authority.pubkey(), 5_000_000_000).unwrap();

    // Data needed for the instruction.
    let fee = 5u16;
    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&fee.to_le_bytes());
    // State of the protocol to show whether it has been initialized.
    instruction_data.push(1u8);

    let accounts = vec![
        AccountMeta::new(authority.pubkey(), true),
        AccountMeta::new(treasury, false),

        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    let instruction = Instruction::new_with_bytes(program_id, &instruction_data, accounts);
    let tx = Transaction::new(
        &[&authority],
        Message::new(&[instruction], Some(&authority.pubkey())),
        svm.latest_blockhash()
    );

    let tx_init = svm.send_transaction(tx);
    //println!("Test initializing the protocol {:#?}", tx_init);

    NeoFlashConfigContext {
        svm, authority, treasury, program_id,
    }
}

// Seting up the test context.
// Here we are setting up the source of liquidity and the destination to simulate transaction.
pub struct TestEnvironment {
    pub liquidity_vault: Pubkey,
    pub liquidity_mint: Pubkey,
    pub liquidity_amount: u64,
}

pub fn init_test_env(mut svm: &mut LiteSVM, program_id: Pubkey) -> TestEnvironment {
    // Generating a mock token mint address and a fee payer.
    let mint_pubkey = Pubkey::new_unique();
    let payer = Keypair::new();
    // Airdropping SOL for fees.
    svm.airdrop(&payer.pubkey(), 2_000_000_00).unwrap();

    // Deriving the protocol PDA authority.
    let (protocol_liquidity_pda, _bump) = Pubkey::find_program_address(&[b"liquidity_pda"], &program_id);

    // Create a new SPL token mint with the payer as the mint authority.
    let mint = CreateMint::new(&mut svm, &payer).authority(&payer.pubkey()).decimals(DECIMALS).send().unwrap();

    let associated_token_account = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint).owner(&protocol_liquidity_pda).send().unwrap();

    // Mint 100000 tokens to Alice's account.
    MintTo::new(&mut svm, &payer, &mint, &associated_token_account, 10_000_000)
        .owner(&payer).send().unwrap();
    // Verify balance.
    let account: TknAccount = get_spl_account(&svm, &associated_token_account).unwrap();
    let balance = account.amount;

    TestEnvironment { 
        liquidity_vault: associated_token_account, 
        liquidity_mint: mint,
        liquidity_amount: balance,
    }
}

pub fn test_borrow_ix(mut loan_ctx: NeoFlashConfigContext, test_ctx: TestEnvironment) {
    let borrower = Keypair::new();
    loan_ctx.svm.airdrop(&borrower.pubkey(), 100_000_000).unwrap();

    // ============== BORROW OPERATION SET UP =================
    // Borrow instruction data.
    let amount = 555u64;
    let mut borrow_instruction_data = vec![1u8];
    borrow_instruction_data.extend_from_slice(&amount.to_le_bytes());

    // ATA for borrower. Stores the borrowed tokens
    let borrower_token_account = CreateAssociatedTokenAccount::new(
        &mut loan_ctx.svm, &borrower, &test_ctx.liquidity_mint
    ).owner(&borrower.pubkey()).send().unwrap();

    // Deriving the pda for config.
    let (config_pda, bump) = Pubkey::find_program_address(
        &[b"config", loan_ctx.treasury.as_ref()], &loan_ctx.program_id
    );

    // Deriving the liquidity vault PDA.
    let (liquidity_vault_pda, _bump) = Pubkey::find_program_address(
        &[b"liquidity_pda"], &loan_ctx.program_id
    );

    // Accounts needed for the borrow instruction are.
    let borrow_accounts = vec![
        AccountMeta::new(borrower.pubkey(), true),
        AccountMeta::new(borrower_token_account, false),
        AccountMeta::new(config_pda, false),
        AccountMeta::new(test_ctx.liquidity_vault, false),
        AccountMeta::new(liquidity_vault_pda, false),
        // This account is only readable and cannot be written.
        AccountMeta::new_readonly(SYSVARS_ID, false),
    ];

    let borrow_instruction = Instruction::new_with_bytes(
        loan_ctx.program_id,
        &borrow_instruction_data,
        borrow_accounts
    );

    // ================ REPAY OPERATION SET UP ==============
    //
    // Repay instruction takes no data. we only have the discriminator.
    let repay_instruction_data = vec![2u8];

    let repay_accounts = vec![
        AccountMeta::new(borrower.pubkey(), true),
        AccountMeta::new(borrower_token_account, false),
        AccountMeta::new(test_ctx.liquidity_vault, false),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(SYSVARS_ID, false),
    ];

    let repay_instruction = Instruction::new_with_bytes(
        loan_ctx.program_id,
        &repay_instruction_data,
        repay_accounts
    );

    // Borrower must sign. It is initiating the loan and paying network fee.
    let tx = Transaction::new(
        &[&borrower],
        Message::new(&[borrow_instruction, repay_instruction], Some(&borrower.pubkey())),
        loan_ctx.svm.latest_blockhash()
    );

    let tx_borrow = loan_ctx.svm.send_transaction(tx);
    println!("The borrow instruction is {:#?}", tx_borrow);
}

