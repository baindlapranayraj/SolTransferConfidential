use anyhow::{Ok, Result};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::{
        extension::{
            confidential_transfer::{
                account_info::TransferAccountInfo, ConfidentialTransferAccount,
            },
            BaseStateWithExtensions,
        },
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    },
    token::{ProofAccountWithCiphertext, Token},
};

pub async fn transfer_tokens(
    amount: u64,
    token: &Token<ProgramRpcClientSendTransaction>,

    sender_token_kp: &Keypair,
    sender_elgamal_kp: &ElGamalKeypair,
    sender_aes_kp: &AeKey,

    sender_kp: &Keypair,

    recipt_elgmal_kp: &ElGamalKeypair,

    recipint_token_kp: &Keypair,
) -> Result<()> {
    // we must generate three types of zero-knowledge proofs to convince the on-chain program that the transfer
    // is correct without revealing any amounts.
    //
    // 1) Equality Proof
    // 2) Ciphertext Validity Proof
    // 3) Range Proof

    let transfer_amount = amount * 10u64.pow(6);
    // Get the token account data to access the confidential transfer extension state
    let token_account = token.get_account_info(&sender_token_kp.pubkey()).await?;

    let extension_data = token_account.get_extension::<ConfidentialTransferAccount>()?;

    // Create TransferAccountInfo from the extension data
    let transfer_account_info = TransferAccountInfo::new(extension_data);

    // Generate the proof data for the transfer
    let transfer_proof_data = transfer_account_info.generate_split_transfer_proof_data(
        transfer_amount,
        &sender_elgamal_kp,
        &sender_aes_kp,
        recipt_elgmal_kp.pubkey(),
        None, // auditor ElGamal public key (none if no auditor)
    )?;

    // Create proof context accounts
    let equality_proof_context_state_keypair = Keypair::new();
    let equality_proof_context_state_pubkey = equality_proof_context_state_keypair.pubkey();

    let ciphertext_validity_proof_context_state_keypair = Keypair::new();
    let ciphertext_validity_proof_context_state_pubkey =
        ciphertext_validity_proof_context_state_keypair.pubkey();

    let range_proof_context_state_keypair = Keypair::new();
    let range_proof_context_state_pubkey = range_proof_context_state_keypair.pubkey();

    // Create context state account for equality proof
    println!("Creating equality proof context state account for transfer...");
    let equality_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_state_pubkey, // Public key of the new equality proof context state account
            &sender_kp.pubkey(), // Authority that can close the context state account
            &transfer_proof_data.equality_proof_data, // Proof data for the equality proof verification
            false, // False: combine account creation and proof verification in one transaction
            &[&equality_proof_context_state_keypair], // Signer for the new account
        )
        .await?;

    println!(
        "Transfer Equality Proof Account Signature: {}",
        equality_proof_signature
    );

    // Create context state account for ciphertext validity proof
    println!("Creating ciphertext validity proof context state account...");
    let ciphertext_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &ciphertext_validity_proof_context_state_pubkey,
            &sender_kp.pubkey(),
            &transfer_proof_data
                .ciphertext_validity_proof_data_with_ciphertext
                .proof_data,
            false,
            &[&ciphertext_validity_proof_context_state_keypair],
        )
        .await?;

    println!(
        "Ciphertext Validity Proof Account Signature: {}",
        ciphertext_proof_signature
    );

    // Create context state account for range proof
    println!("Creating range proof context state account...");
    let range_proof_signature = token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_state_pubkey,
            &sender_kp.pubkey(),
            &transfer_proof_data.range_proof_data,
            true,
            &[&range_proof_context_state_keypair],
        )
        .await?;
    println!("Range Proof Account Signature: {}", range_proof_signature);

    // Execute the confidential transfer
    println!("Executing confidential transfer transaction...");

    // Create a ProofAccountWithCiphertext for the ciphertext validity proof
    let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
        context_state_account: ciphertext_validity_proof_context_state_pubkey,

        ciphertext_lo: transfer_proof_data
            .ciphertext_validity_proof_data_with_ciphertext
            .ciphertext_lo,
        ciphertext_hi: transfer_proof_data
            .ciphertext_validity_proof_data_with_ciphertext
            .ciphertext_hi,
    };

    let transfer_signature = token
        .confidential_transfer_transfer(
            &sender_token_kp.pubkey(),
            &recipint_token_kp.pubkey(),
            &sender_kp.pubkey(),
            Some(&equality_proof_context_state_pubkey),
            Some(&ciphertext_validity_proof_account_with_ciphertext),
            Some(&range_proof_context_state_pubkey),
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

    Ok(())
}
