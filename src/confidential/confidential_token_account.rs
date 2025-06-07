use anyhow::{Ok, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer, system_instruction};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::{
        self,
        extension::{
            confidential_transfer::instruction::{configure_account, PubkeyValidityProofData},
            ExtensionType,
        },
        instruction::initialize_account3,
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
        state::Account,
    },
    token::Token,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;

use crate::helper::{complete_ixs, ConfTokenAccountRes};

/// Creates a new confidential token account with the ConfidentialTransfer extension enabled.
///
/// # Arguments
/// * `payer` - The keypair paying for account creation and rent.
/// * `mint_kp` - The mint keypair for the token.
/// * `rpc_client` - The Solana RPC client.
/// * `token` - The SPL Token client.
///
/// # Returns
/// * `ConfTokenAccountRes` - Struct containing the new token account keypair and cryptographic keys.
pub async fn create_confidential_token_acc(
    payer: &Keypair,
    mint_kp: &Keypair,
    rpc_client: &RpcClient,
    token: &Token<ProgramRpcClientSendTransaction>,
) -> Result<ConfTokenAccountRes> {
    println!("\n======== Creating New Confidential Token Account ========");
    // Generate a new keypair for the user's token account
    let token_account_kp = Keypair::new();
    println!("Generated new token account: {}", token_account_kp.pubkey());

    println!("Generating cryptographic keys for confidential transactions...");
    // Generate ElGamal and AES keys for confidential encryption, unique to this account
    let elgamal_kp = ElGamalKeypair::new_from_signer(&payer, &token_account_kp.pubkey().to_bytes())
        .expect("Unable to create Elgamal KP");
    println!("Created ElGamal keypair for confidential encryption");
    
    let aes_kp = AeKey::new_from_signer(&payer, &token_account_kp.pubkey().to_bytes())
        .expect("Unable to create AES KP");
    println!("Created AES key for confidential encryption");

    println!("\nCalculating account space and rent requirements...");
    // Calculate the required space for the account with the ConfidentialTransfer extension
    let required_space = ExtensionType::try_calculate_account_len::<Account>(&[
        ExtensionType::ConfidentialTransferAccount,
    ])?;
    println!("Required account space: {} bytes", required_space);

    // Get the minimum balance needed to make the account rent-exempt
    let rent_req = rpc_client
        .get_minimum_balance_for_rent_exemption(required_space)
        .await?;
    println!("Required rent (lamports): {}", rent_req);

    // Instruction to create the new token account
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account_kp.pubkey(),
        rent_req,
        required_space as u64,
        &spl_token_2022::ID,
    );

    // Instruction to initialize the token account for the given mint
    let intialize_token_account_ix = initialize_account3(
        &spl_token_2022::ID,
        &token_account_kp.pubkey(),
        &mint_kp.pubkey(),
        &payer.pubkey(),
    )?;

    // Generate a ZK proof to prove the validity of the ElGamal public key
    let proof_data = PubkeyValidityProofData::new(&elgamal_kp)
        .map_err(|_| anyhow::anyhow!("Failed to generate proof data"))?;

    // Specify where the proof data is located in the transaction
    let proof_location = ProofLocation::InstructionOffset(1.try_into()?, &proof_data);

    // Instruction to configure the confidential transfer extension for the account
    let confidential_transfer_account_ix = configure_account(
        &spl_token_2022::id(),
        &token_account_kp.pubkey(),
        &mint_kp.pubkey(),
        &aes_kp.encrypt(0).into(), // Initial encrypted balance is zero
        65536,                     // Maximum pending balance credit counter
        &payer.pubkey(),
        &[],
        proof_location,
    )?;

    // Combine all instructions into a single transaction
    let mut ix = vec![create_account_ix, intialize_token_account_ix];
    ix.extend(confidential_transfer_account_ix);

    // Submit the transaction to create and configure the confidential token account
    complete_ixs(rpc_client, ix, &[&payer, &token_account_kp], &payer).await?;

    // Enable confidential transfers for the new token account
    token
        .confidential_transfer_enable_confidential_credits(
            &token_account_kp.pubkey(),
            &payer.pubkey(),
            &[&payer, &token_account_kp],
        )
        .await?;

    // Return the new account and its cryptographic keys
    let res = ConfTokenAccountRes {
        token_account_kp,
        user_elgamal_kp: elgamal_kp,
        user_aes_kp: aes_kp,
    };

    Ok(res)
}
