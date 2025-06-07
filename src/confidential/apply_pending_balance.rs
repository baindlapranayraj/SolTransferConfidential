use anyhow::{Ok, Result};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    token::Token,
};

use crate::helper::handle_token_response;

/// Applies the pending confidential balance to the available balance for a token account.
/// This is required after a confidential transfer or deposit to make the tokens usable.
pub async fn apply_pending(
    token: &Token<ProgramRpcClientSendTransaction>,
    payer: &Keypair,                // The account paying for the transaction fees
    elgamal_kp: &ElGamalKeypair,    // ElGamal keypair for decrypting the confidential balance
    aes_kp: &AeKey,                 // AE key for decrypting the confidential balance
    token_account_kp: &Keypair,     // The confidential token account
) -> Result<()> {
    println!("\n======== Converting Pending Balance to Available Balance ========");
    println!("Account: {}", token_account_kp.pubkey());
    println!("Authority: {}", payer.pubkey());
    println!("\nStep 1: Decrypting pending balance using account's cryptographic keys...");
    
    let apply_sig = token
        .confidential_transfer_apply_pending_balance(
            &token_account_kp.pubkey(),
            &payer.pubkey(),
            None,
            elgamal_kp.secret(),
            aes_kp,
            &[payer],
        )
        .await?;

    handle_token_response(&apply_sig, String::from("applying pending account")).await?;

    println!("\nStep 2: Converting decrypted pending balance to available balance...");
    println!("✓ Successfully moved pending balance to available balance");
    println!("Note: The available balance is encrypted and can only be viewed by the account owner");
    Ok(())
}
