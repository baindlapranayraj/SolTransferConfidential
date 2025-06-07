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

pub async fn create_confidential_token_acc(
    payer: &Keypair,
    mint_kp: &Keypair,
    rpc_client: &RpcClient,
    token: &Token<ProgramRpcClientSendTransaction>,
) -> Result<ConfTokenAccountRes> {
    // User Token Keypair
    let token_account_kp = Keypair::new();

    // Create Elgamal and AES key for token account.
    let elgamal_kp = ElGamalKeypair::new_from_signer(&payer, &token_account_kp.pubkey().to_bytes())
        .expect("Unable to create Elgamal KP");
    let aes_kp = AeKey::new_from_signer(&payer, &token_account_kp.pubkey().to_bytes())
        .expect("Unable to create AES KP");

    let required_space = ExtensionType::try_calculate_account_len::<Account>(&[
        ExtensionType::ConfidentialTransferAccount,
    ])?;

    let rent_req = rpc_client
        .get_minimum_balance_for_rent_exemption(required_space)
        .await?;

    // Create Token Account with ConfidentialTransfer Extension (using system_instruction)
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account_kp.pubkey(),
        rent_req,
        required_space as u64,
        &spl_token_2022::ID,
    );

    // For initializing the Token account manually
    let intialize_token_account_ix = initialize_account3(
        &spl_token_2022::ID,
        &token_account_kp.pubkey(),
        &mint_kp.pubkey(),
        &payer.pubkey(),
    )?;

    // A ZK Proof needed for creating the ConfidentialTransferAccount
    let proof_data = PubkeyValidityProofData::new(&elgamal_kp)
        .map_err(|_| anyhow::anyhow!("Failed to generate proof data"))?;

    // This tells the on-chain program where in your transaction it can find the proof data.
    let proof_location = ProofLocation::InstructionOffset(1.try_into()?, &proof_data);

    let confidential_transfer_account_ix = configure_account(
        &spl_token_2022::id(),
        &token_account_kp.pubkey(),
        &mint_kp.pubkey(),
        &aes_kp.encrypt(0).into(),
        65536,
        &payer.pubkey(),
        &[],
        proof_location,
    )?;

    // All Instructions Combined
    let mut ix = vec![create_account_ix, intialize_token_account_ix];
    ix.extend(confidential_transfer_account_ix);

    complete_ixs(rpc_client, ix, &[&payer, &token_account_kp], &payer).await?;

    // Enable confidential transfers for the token account
    token
        .confidential_transfer_enable_confidential_credits(
            &token_account_kp.pubkey(),
            &payer.pubkey(),
            &[&payer, &token_account_kp],
        )
        .await?;

    let res = ConfTokenAccountRes {
        token_account_kp,
        user_elgamal_kp: elgamal_kp,
        user_aes_kp: aes_kp,
    };

    Ok(res)
}
