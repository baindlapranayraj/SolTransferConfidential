use anyhow::{Ok, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    token::{ExtensionInitializationParams, Token},
};

use crate::helper::handle_token_response;

/// Creates a new confidential mint with the ConfidentialTransfer extension enabled.
///
/// # Arguments
/// * `mint_authority` - The public key that will have minting authority.
/// * `signers` - The keypairs required to sign the mint creation transaction.
/// * `token` - The SPL Token client.
///
/// # Flow
/// 1. Sets up the ConfidentialTransfer extension parameters (authority, auto-approve, no auditor).
/// 2. Calls the SPL Token client to create the mint with the extension.
/// 3. Prints the transaction signature or logs.
pub async fn create_confidential_mint(
    mint_authority: &Pubkey,
    signers: &[&Keypair],
    token: &Token<ProgramRpcClientSendTransaction>,
) -> Result<()> {
    // Set up the ConfidentialTransfer extension parameters for the mint
    let extension_initialization_params = ExtensionInitializationParams::ConfidentialTransferMint {
        authority: Some(*mint_authority),           // Set the mint authority
        auto_approve_new_accounts: true,            // Automatically approve new confidential accounts
        auditor_elgamal_pubkey: None,               // No global auditor for this confidential mint
    };

    // Create the mint account with the ConfidentialTransfer extension
    let create_mint_sig = token
        .create_mint(
            mint_authority,                        // Mint authority - can mint new tokens
            Some(mint_authority),                  // Freeze authority - can freeze token accounts
            vec![extension_initialization_params], // Add the ConfidentialTransferMint extension
            &[signers[0], signers[1]],             // Mint keypair(s) needed as signer(s)
        )
        .await?;

    // Print transaction signature or logs
    handle_token_response(
        &create_mint_sig,
        String::from("creating confidential mint account"),
    )
    .await?;

    Ok(())
}
