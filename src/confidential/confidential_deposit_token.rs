use anyhow::{Ok, Result};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_client::{
    client::ProgramRpcClientSendTransaction,
    spl_token_2022::solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    token::Token,
};

use crate::helper::handle_token_response;

use super::apply_pending;

pub async fn deposite_token_to_confidential(
    token_account_kp: &Keypair,
    payer: &Keypair,
    token: &Token<ProgramRpcClientSendTransaction>,
    elgamal_kp: &ElGamalKeypair,
    aes_kp: &AeKey,
) -> Result<()> {
    // Confidential balance has separate "pending" and "available" balances
    //
    // 1) First we deposite our tokens to pending account
    // 2) Second we deposite our tokens to available account

    // 1) Deposit tokens  balance to  "pending" confidential balance
    let deposit_sig = token
        .confidential_transfer_deposit(
            &token_account_kp.pubkey(),
            &payer.pubkey(),
            100 * 10u64.pow(6),
            6,
            &[payer],
        )
        .await?;

    handle_token_response(&deposit_sig, String::from("deposit tokens to pending")).await?;

    // 2) Apply the "pending" balance to "available" balances
    apply_pending(&token, &payer, &elgamal_kp, &aes_kp, &token_account_kp).await?;

    Ok(())
}
