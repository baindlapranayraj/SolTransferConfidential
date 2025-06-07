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
    println!("\n======== Creating Connection to Local Solana RPC ========");
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        String::from("http://localhost:8899"),
        CommitmentConfig::confirmed(),
    ));
    println!("Connected to Solana RPC at localhost:8899");

    println!("\n======== Generating Funded Keypairs for Alice and Bob ========");
    let bob = keypair_gen(&rpc_client).await?;
    println!("Generated Bob's keypair: {}", bob.pubkey());
    let alice = keypair_gen(&rpc_client).await?;
    println!("Generated Alice's keypair: {}", alice.pubkey());

    println!("\n======== Creating New Mint Account ========");
    let mint_kp = Keypair::new(); // Mint Keypair
    println!("Generated mint keypair: {}", mint_kp.pubkey());

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

    // ======== Create Mint Account with ConfidentialTransferMint extension ======== 
    create_confidential_mint(&alice.pubkey(), &[&mint_kp, &alice], &token).await?;

    println!("\n========  Configure token account created for bob and alice ======= \n");
    let alice_res = create_confidential_token_acc(&alice, &mint_kp, &rpc_client, &token).await?;
    let bob_res = create_confidential_token_acc(&bob, &mint_kp, &rpc_client, &token).await?;

    // Print initial balances for Alice and Bob
    println!("[Before Mint] Fetching Alice's confidential token account balance...");
    fetch_token_account_with_extensions(&rpc_client, &alice_res.token_account_kp.pubkey()).await?;
    
    // ======== Minting some tokens for alice token account ========
    token
        .mint_to(
            &alice_res.token_account_kp.pubkey(), // Destination
            &alice.pubkey(),                      // Token Account authority
            100 * 10u64.pow(6),                   // Miniting 100 Tokens
            &[&alice],                            // Signers
        )
        .await?;

    fetch_token_account_with_extensions(&rpc_client, &alice_res.token_account_kp.pubkey()).await?;

    // Depositing tokens for alice pending account and apply pending account to available balance
    deposite_token_to_confidential(
        &alice_res.token_account_kp,
        &alice,
        &token,
        &alice_res.user_elgamal_kp,
        &alice_res.user_aes_kp,
    )
    .await?;

    fetch_token_account_with_extensions(&rpc_client, &alice_res.token_account_kp.pubkey()).await?;

    // Transfer Tokens Confidentially Alice to Bob
    transfer_tokens(
        50,
        &token,
        &alice_res.token_account_kp,
        &alice_res.user_elgamal_kp,
        &alice_res.user_aes_kp,
        &alice,
        &bob,
        &bob_res.user_elgamal_kp,
        &bob_res.user_aes_kp,
        &bob_res.token_account_kp,
    )
    .await?;


    Ok(())
}
