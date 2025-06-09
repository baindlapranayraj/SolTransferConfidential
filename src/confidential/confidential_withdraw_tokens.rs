use anyhow::{Ok, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::{
        extension::{
            confidential_transfer::{account_info::WithdrawAccountInfo, ConfidentialTransferAccount},
            BaseStateWithExtensions,
        },
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    },
    token::Token,
};
use spl_token_confidential_transfer_proof_generation::withdraw::WithdrawProofData;

use crate::helper::handle_token_response;

/// Withdraws tokens from a confidential account, proving correctness with ZK proofs.
///
/// # Arguments
/// * `token_pubkey` - The confidential token account public key.
/// * `elgmal_kp` - ElGamal keypair for confidential encryption.
/// * `aes_key` - AE key for confidential encryption.
/// * `amount` - Amount to withdraw (in base units).
/// * `token` - The SPL Token client.
/// * `user_kp` - The user's main keypair (authority).
///
/// # Flow
/// 1. Fetches the confidential account extension data.
/// 2. Generates ZK proofs (equality and range) for the withdrawal.
/// 3. Creates context state accounts for each proof.
/// 4. Executes the confidential withdrawal referencing the proof accounts.
/// 5. Closes all proof context state accounts to reclaim rent.
pub async fn withdraw_tokens(
    token_pubkey: &Pubkey,
    elgmal_kp: &ElGamalKeypair,
    aes_key: &AeKey,
    amount: u64,
    token: &Token<ProgramRpcClientSendTransaction>,
    user_kp: &Keypair,
) -> Result<()> {
    // Get the token account data to access the confidential transfer extension
    let token_accountinfo = token.get_account_info(token_pubkey).await?;
    let extension_data = token_accountinfo.get_extension::<ConfidentialTransferAccount>()?;

    // Prepare withdrawal account info for proof generation
    let withdraw_accountinfo = WithdrawAccountInfo::new(extension_data);

    // Create keypairs for the proof context state accounts
    let equality_proof_context_state_keypair = Keypair::new();
    let equality_proof_context_state_pubkey = equality_proof_context_state_keypair.pubkey();
    let range_proof_context_state_keypair = Keypair::new();
    let range_proof_context_state_pubkey = range_proof_context_state_keypair.pubkey();

    // Generate the ZK proof data for withdrawal (equality and range proofs)
    let WithdrawProofData {
        equality_proof_data,
        range_proof_data,
    } = withdraw_accountinfo.generate_proof_data(
        amount * 10u64.pow(6), // Amount to withdraw (adjust for decimals)
        &elgmal_kp,            // ElGamal keypair for encryption
        &aes_key,              // AES key for encryption
    )?;

    // Create context state account for equality proof
    println!("Create equality proof context state account");
    let equality_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_state_pubkey,
            &user_kp.pubkey(),
            &equality_proof_data,
            false,
            &[&equality_proof_context_state_keypair],
        )
        .await?;
    println!(
        "Equality Proof Context State Account Signature: {}",
        equality_proof_signature
    );

    // Create context state account for range proof
    println!("Create range proof context state account");
    let range_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_state_pubkey,
            &user_kp.pubkey(),
            &range_proof_data,
            true, // True: split account creation and proof verification for large proofs
            &[&range_proof_context_state_keypair],
        )
        .await?;
    println!(
        "Range Proof Context State Account Signature: {}",
        range_proof_signature
    );

    // Execute the confidential withdrawal referencing the proof accounts
    println!("\n======== Preparing Confidential Withdraw ========");
    let withdraw_sig = token
        .confidential_transfer_withdraw(
            token_pubkey,
            &user_kp.pubkey(),
            Some(&equality_proof_context_state_pubkey),
            Some(&range_proof_context_state_pubkey),
            amount * 10u64.pow(6), // Withdraw amount (adjust for decimals)
            6,                     // Token decimals
            Some(withdraw_accountinfo),
            &elgmal_kp,
            &aes_key,
            &[&user_kp],
        )
        .await?;

    handle_token_response(&withdraw_sig, String::from("confidential withdraw amount")).await?;

    // Close all proof context state accounts to reclaim rent
    println!("Closing all proof context state account...");
    token
        .confidential_transfer_close_context_state_account(
            &equality_proof_context_state_pubkey,
            &user_kp.pubkey(),
            &user_kp.pubkey(),
            &[&user_kp],
        )
        .await?;

    token
        .confidential_transfer_close_context_state_account(
            &range_proof_context_state_pubkey,
            &user_kp.pubkey(),
            &user_kp.pubkey(),
            &[&user_kp],
        )
        .await?;

    println!("Closed all context state accounts");

    Ok(())
}
