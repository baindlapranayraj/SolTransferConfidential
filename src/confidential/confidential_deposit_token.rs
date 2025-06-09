use anyhow::{Ok, Result};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    token::Token,
};

use super::apply_pending;
use crate::helper::handle_token_response;

/// Deposits tokens into a confidential account.
///
/// # Arguments
/// * `token_account_kp` - The confidential token account keypair.
/// * `payer` - The keypair paying for the transaction.
/// * `token` - The SPL Token client.
/// * `elgamal_kp` - ElGamal keypair for confidential encryption.
/// * `aes_kp` - AE key for confidential encryption.
///
/// # Flow
/// 1. Deposit tokens to the 'pending' confidential balance.
/// 2. Apply the 'pending' balance to make it available for spending.
pub async fn deposite_token_to_confidential(
    token_account_kp: &Keypair,
    payer: &Keypair,
    token: &Token<ProgramRpcClientSendTransaction>,
    elgamal_kp: &ElGamalKeypair,
    aes_kp: &AeKey,

    amount: u64,
) -> Result<()> {
    println!("\n======== Depositing Tokens to Confidential Account ========");
    println!("Note: Confidential transfers use a two-step process:");
    println!("1. Deposit to 'pending' balance");
    println!("2. Apply pending to 'available' balance");

    // Step 1: Deposit tokens to the 'pending' confidential balance.
    println!("\nStep 1: Depositing 100 tokens to pending balance...");
    println!("- Token Account: {}", token_account_kp.pubkey());
    println!("- Amount: {} tokens ", amount);

    let deposit_sig = token
        .confidential_transfer_deposit(
            &token_account_kp.pubkey(),
            &payer.pubkey(),
            amount * 10u64.pow(6), // Amount to deposit (adjust for decimals)
            6,                     // Token decimals
            &[payer],
        )
        .await?;

    handle_token_response(&deposit_sig, String::from("deposit tokens to pending")).await?;

    // Step 2: Apply the 'pending' balance to make it available for spending.
    println!("\nStep 2: Converting pending balance to available balance...");
    println!("- Token Account: {}", token_account_kp.pubkey());
    apply_pending(&token, &payer, &elgamal_kp, &aes_kp, &token_account_kp).await?;
    println!("✓ Successfully converted pending balance to available balance");

    Ok(())
}
