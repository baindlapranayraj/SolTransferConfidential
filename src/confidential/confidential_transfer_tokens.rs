use anyhow::{Ok, Result};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::{
        extension::{
            confidential_transfer::{account_info::TransferAccountInfo, ConfidentialTransferAccount},
            BaseStateWithExtensions,
        },
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    },
    token::{ProofAccountWithCiphertext, Token},
};

use crate::confidential::apply_pending;

/// Performs a confidential token transfer using ZK proofs and applies the pending balance to the recipient.
///
/// # Arguments
/// * `amount` - The amount to transfer (in base units, e.g., 1 = 1 token if decimals=0)
/// * `token` - The SPL Token client
/// * `sender_token_kp` - Sender's confidential token account keypair
/// * `sender_elgamal_kp` - Sender's ElGamal keypair for encryption
/// * `sender_aes_kp` - Sender's AE key for encryption
/// * `sender_kp` - Sender's main keypair (authority)
/// * `recipint_kp` - Recipient's main keypair
/// * `recipt_elgmal_kp` - Recipient's ElGamal keypair
/// * `recipt_aes_kp` - Recipient's AE key
/// * `recipint_token_kp` - Recipient's confidential token account keypair
///
/// # Flow
/// 1. Generates three ZK proofs: equality, validity, and range.
/// 2. Creates context state accounts for each proof.
/// 3. Executes the confidential transfer referencing the proof accounts.
/// 4. Applies the pending balance to the recipient's available balance.
/// 5. Closes all proof context state accounts to reclaim rent.
pub async fn transfer_tokens(
    amount: u64,
    token: &Token<ProgramRpcClientSendTransaction>,

    sender_token_kp: &Keypair,
    sender_elgamal_kp: &ElGamalKeypair,
    sender_aes_kp: &AeKey,
    sender_kp: &Keypair,

    recipint_kp: &Keypair,
    recipt_elgmal_kp: &ElGamalKeypair,
    recipt_aes_kp: &AeKey,
    recipint_token_kp: &Keypair,
) -> Result<()> {
    // Generate three types of zero-knowledge proofs to convince the on-chain program that the transfer is correct without revealing any amounts.
    // 1) Equality Proof: Proves the transferred amount is the same for sender and recipient.
    // 2) Ciphertext Validity Proof: Proves the ciphertexts are valid encryptions.
    // 3) Range Proof: Proves the transferred amount is within a valid range.

    let transfer_amount = amount * 10u64.pow(6); // Adjust for token decimals

    // Get the token account data (contains both token base account and confidential account)
    let token_account = token.get_account_info(&sender_token_kp.pubkey()).await?;

    // Extract the confidential transfer extension data from the token account data
    let extension_data = token_account.get_extension::<ConfidentialTransferAccount>()?;

    // Create TransferAccountInfo from the extension data
    let transfer_account_info = TransferAccountInfo::new(extension_data);

    // Generate the proof data for the transfer (all ZKPs required for a confidential transfer)
    let transfer_proof_data = transfer_account_info.generate_split_transfer_proof_data(
        transfer_amount,
        &sender_elgamal_kp,
        &sender_aes_kp,
        recipt_elgmal_kp.pubkey(),
        None, // auditor ElGamal public key (none if no auditor)
    )?;

    println!("\n======== Preparing Confidential Transfer ========");
    println!("Transfer Details:");
    println!("- Amount: {} tokens", amount);
    println!("- From: {}", sender_token_kp.pubkey());
    println!("- To: {}", recipint_token_kp.pubkey());

    println!("\nGenerating Zero-Knowledge Proofs...");
    println!("Creating proof context state accounts:");
    
    // Create context state accounts for each proof
    let equality_proof_context_state_keypair = Keypair::new();  // Equality Proof
    let ciphertext_validity_proof_context_state_keypair = Keypair::new();  // Validity Proof
    let range_proof_context_state_keypair = Keypair::new();  // Range Proof

    // Create context state account for equality proof
    println!("1. Creating Equality Proof (proves transferred amount is the same for sender and recipient)...");
    token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_state_keypair.pubkey(),
            &sender_kp.pubkey(),
            &transfer_proof_data.equality_proof_data,
            false,
            &[&equality_proof_context_state_keypair],
        )
        .await?;
    println!("   ✓ Equality proof created");

    // Create context state account for ciphertext validity proof
    println!("2. Creating Ciphertext Validity Proof (proves the encrypted amounts are valid)...");
    token
        .confidential_transfer_create_context_state_account(
            &ciphertext_validity_proof_context_state_keypair.pubkey(),
            &sender_kp.pubkey(),
            &transfer_proof_data.ciphertext_validity_proof_data_with_ciphertext.proof_data,
            false,
            &[&ciphertext_validity_proof_context_state_keypair],
        )
        .await?;
    println!("   ✓ Ciphertext validity proof created");

    // Create context state account for range proof
    println!("3. Creating Range Proof (proves the transfer amount is within valid range)...");
    token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_state_keypair.pubkey(),
            &sender_kp.pubkey(),
            &transfer_proof_data.range_proof_data,
            true,
            &[&range_proof_context_state_keypair],
        )
        .await?;
    println!("   ✓ Range proof created");

    // Execute the confidential transfer
    println!("Executing confidential transfer transaction...");
    let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
        context_state_account: ciphertext_validity_proof_context_state_keypair.pubkey(),
        ciphertext_lo: transfer_proof_data.ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
        ciphertext_hi: transfer_proof_data.ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
    };

    let transfer_signature = token
        .confidential_transfer_transfer(
            &sender_token_kp.pubkey(),
            &recipint_token_kp.pubkey(),
            &sender_kp.pubkey(),
            Some(&equality_proof_context_state_keypair.pubkey()),
            Some(&ciphertext_validity_proof_account_with_ciphertext),
            Some(&range_proof_context_state_keypair.pubkey()),
            transfer_amount,
            None,
            &sender_elgamal_kp,
            &sender_aes_kp,
            recipt_elgmal_kp.pubkey(),
            None,
            &[&sender_kp],
        )
        .await?;

    println!("Confidential Transfer Signature: {}", transfer_signature);

    // Apply the pending balance to the recipient's available balance
    apply_pending(
        &token,
        &recipint_kp,
        &recipt_elgmal_kp,
        &recipt_aes_kp,
        &recipint_token_kp,
    ).await?;

    // Close all proof context state accounts to reclaim rent
    println!("Closing all proof context state account...");
    token.confidential_transfer_close_context_state_account(
        &equality_proof_context_state_keypair.pubkey(),
        &sender_kp.pubkey(),
        &sender_kp.pubkey(),
        &[&sender_kp],
    ).await?;
    token.confidential_transfer_close_context_state_account(
        &ciphertext_validity_proof_context_state_keypair.pubkey(),
        &sender_kp.pubkey(),
        &sender_kp.pubkey(),
        &[&sender_kp],
    ).await?;
    token.confidential_transfer_close_context_state_account(
        &range_proof_context_state_keypair.pubkey(),
        &sender_kp.pubkey(),
        &sender_kp.pubkey(),
        &[&sender_kp],
    ).await?;
    println!("Closed all context state accounts");

    Ok(())
}
