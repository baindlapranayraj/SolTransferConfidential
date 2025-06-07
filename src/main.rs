use anyhow::{Ok, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use spl_token_client::{
    client::{ProgramRpcClient, ProgramRpcClientSendTransaction},
    spl_token_2022::{self},
    token::Token,
};
use std::sync::Arc;

pub mod helper;
use helper::*;

pub mod confidential;
use confidential::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create Connection to local RPC
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        String::from("http://localhost:8899"),
        CommitmentConfig::confirmed(),
    ));

    // Keypairs with funded wallet
    let bob = keypair_gen(&rpc_client).await?;
    let alice = keypair_gen(&rpc_client).await?;

    let mint_kp = Keypair::new(); // Mint Keypair

    // To interact with solana programs
    let program_client = ProgramRpcClient::new(rpc_client.clone(), ProgramRpcClientSendTransaction);

    // Helps us to interact with spl-token-programs
    let token = Token::new(
        Arc::new(program_client),         // Program Client
        &spl_token_2022::ID,              // SPL Token Program 2022 Publickey
        &mint_kp.pubkey(),                // Mint Address
        Some(6),                          // Mint Decimal
        Arc::new(alice.insecure_clone()), // Payer
    );

    println!("\n ======== Create Mint Account with ConfidentialTransferMint extension ======== \n");
    create_confidential_mint(&alice.pubkey(), &[&mint_kp, &alice], &token).await?;

    println!("\n========  Configure token account created for bob and alice ======= \n");
    let alice_res = create_confidential_token_acc(&alice, &mint_kp, &rpc_client, &token).await?;
    let bob_res = create_confidential_token_acc(&bob, &mint_kp, &rpc_client, &token).await?;

    // Minting some tokens for alice token account
    token
        .mint_to(
            &alice_res.token_account_kp.pubkey(), // Destination
            &alice.pubkey(),                      // Token Account authority
            100 * 10u64.pow(6),                   // Miniting 100 Tokens
            &[&alice],                            // Signers
        )
        .await?;

    // Depositing tokens for alice pending account and apply pending account to available balance
    deposite_token_to_confidential(
        &alice_res.token_account_kp,
        &alice,
        &token,
        &alice_res.user_elgamal_kp,
        &alice_res.user_aes_kp,
    )
    .await?;

    // Transfer Tokens Confidentially Alice to Bob
    transfer_tokens(
        50,
        &token,
        &alice_res.token_account_kp,
        &alice_res.user_elgamal_kp,
        &alice_res.user_aes_kp,
        &alice,
        &bob_res.user_elgamal_kp,
        &bob_res.token_account_kp,
    )
    .await?;

    // Withdraw Tokens Confidentially

    Ok(())
}
