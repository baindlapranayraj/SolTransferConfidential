use anyhow::{Ok, Result};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    token::{ExtensionInitializationParams, Token},
};

use crate::helper::handle_token_response;

pub async fn create_confidential_mint(
    mint_authority: &Pubkey,
    signers: &[&Keypair],
    token: &Token<ProgramRpcClientSendTransaction>,
) -> Result<()> {
    let extension_initialization_params = ExtensionInitializationParams::ConfidentialTransferMint {
        authority: Some(*mint_authority),
        auto_approve_new_accounts: true,
        auditor_elgamal_pubkey: None, // There is no Global Auditor for this Confidentail Transfer
    };

    // Initialize mint account with confidential transer extension
    let create_mint_sig = token
        .create_mint(
            mint_authority,                        // Mint authority - can mint new tokens
            Some(mint_authority),                  // Freeze authority - can freeze token accounts
            vec![extension_initialization_params], // Add the ConfidentialTransferMint extension
            &[signers[0], signers[1]],             // Mint keypair needed as signer
        )
        .await?;

    handle_token_response(
        &create_mint_sig,
        String::from("creating confidential mint account"),
    )
    .await?;

    Ok(())
}
