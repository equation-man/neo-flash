//! Main test file to test for flash loan transactions.
#[allow(warnings)]
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    signature::{ Signer },
};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
mod setup_helpers;
use crate::setup_helpers::{
    NeoFlashTestContext, setup_neo_flash_context,
    get_token_balance,
};

#[test]
fn test_flashloan_success() {
    // Testing a successful flash loan.
    let mut ctx = setup_neo_flash_context();

    // ======================== BORROW INSTRUCTION ================================
    // Instruction data preparation.
    // specify bump, fee, fee, amounts.
    let fee_bps: u16 = 50; // 0.5% fee
    // deriving the bump.
    let (borrow_pda, bump) = Pubkey::find_program_address(
        &[b"loan", ctx.protocol.pubkey().as_ref()],
        &ctx.program_id,
    );
    let borrow_amounts = vec![1_000_000u64]; // 1 USDC (if 6 decimals)
    let mut borrow_data = Vec::new();
    borrow_data.push(0u8);
    borrow_data.push(bump);
    borrow_data.extend_from_slice(&fee_bps.to_le_bytes());
    for &amt in &borrow_amounts {
        borrow_data.extend_from_slice(&amt.to_le_bytes());
    }
    
    
    // Defining the account pairs. i.e [protocol_vault, borrower_token,..]
    let mut account_metas = vec![
        AccountMeta::new(ctx.borrower.pubkey(), true),
        AccountMeta::new_readonly(ctx.protocol.pubkey(), false),
        AccountMeta::new(borrow_pda, false), // Scratch PDA
        AccountMeta::new_readonly(solana_program::sysvar::instructions::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
    ];

    // Add the token account pairs.
    account_metas.push(AccountMeta::new(ctx.protocol_token, false));
    account_metas.push(AccountMeta::new(ctx.borrower_token, false));

    // Building loan instructions.
    let borrow_ix = Instruction {
        program_id: ctx.program_id, accounts: account_metas, data: borrow_data
    };

    // Setup SPL Transfer Instruction. The actual money movement back
    // principal + fee (50 bps)
    let repay_amount = 1_000_000 + (1_000_000 * 50 / 10_000);
    let spl_transfer_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &ctx.borrower_token,
        &ctx.protocol_token,
        &ctx.borrower.pubkey(),
        &[],
        repay_amount,
    ).unwrap();

    // ==================================== REPAY INSTRUCTION ===========================
    // Setup Repay Verification Instruction. Program's Repay logic
    let repay_ix = Instruction {
        program_id: ctx.program_id,
        accounts: vec![
            AccountMeta::new(ctx.borrower.pubkey(), true),
            AccountMeta::new(ctx.loan.pubkey(), false),
            // Repay logic expects only protocol vaults in its trailing slice
            AccountMeta::new_readonly(ctx.protocol_token, false),
        ],
        data: vec![1], // Discriminator for Repay
    };
    println!("The loan address is {}", ctx.loan.pubkey());
    // Wrappin in transaction and sending.
    // We add both the borrow and repay transaction to this same array
    let tx = Transaction::new_signed_with_payer(
        &[borrow_ix, spl_transfer_ix, repay_ix],
        Some(&ctx.borrower.pubkey()),
        &[&ctx.borrower],
        ctx.svm.latest_blockhash()
    );

    // Execute and Assert
    let result = ctx.svm.send_transaction(tx);
    println!("The test result is {:#?}", result);
    //assert!(result.is_ok(), "Transaction failed {:?}", result.err()); // Verify success
    
    // Verify the loan account was closed and lamports returned.
    let loan_account_after = ctx.svm.get_account(&ctx.loan.pubkey());
    //assert!(loan_account_after.is_none(), "Loan account should be closed and deleted.");
    println!("The account after loan is {:#?}", loan_account_after);
}
