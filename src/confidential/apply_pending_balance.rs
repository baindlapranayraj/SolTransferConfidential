use anyhow::{Ok, Result};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    token::Token,
};

use crate::helper::handle_token_response;

pub async fn apply_pending(
    token: &Token<ProgramRpcClientSendTransaction>,
    payer: &Keypair,
    elgamal_kp: &ElGamalKeypair,
    aes_kp: &AeKey,
    token_account_kp: &Keypair,
) -> Result<()> {
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

    Ok(())
}
