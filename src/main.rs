use anyhow::{Ok, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use spl_token_client::{
    client::{ProgramRpcClient, ProgramRpcClientSendTransaction},
    spl_token_2022::{self},
    token::Token,
};
use std::{io::stdin, sync::Arc};

pub mod helper;
use helper::*;

pub mod confidential;
use confidential::*;

//
// Common stuff :
//  - RPC connect
//  - Alice and Bob keypair generation
//  - Confidential Mint Account
//  - Confidential Token Account for Alice and Bob
//
// ++++++++++++++++++++++++++++++++++++  CLI stuff ++++++++++++++++++++++++++++++++++++
//  match input {
//   check_token_account =>{
//     alice =>{},
//     bob =>{}
//   },
//
//   mint_tokens =>{},
//
//   confidential_deposite_pending => {},
//   confidential_transfer_tokens => {},
//   confidential_withdraw_tokens =>{}
//  }

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
    let alice = keypair_gen(&rpc_client).await?;

    println!(
        "Generated Alice's and Bob's keypair: {} and {}",
        alice.pubkey(),
        bob.pubkey()
    );

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

    loop {
        println!("\n================== 📝 Choose an instruction ==================\n");
        println!("1️⃣  Check Token Account");
        println!("2️⃣  Mint Tokens");
        println!("3️⃣  Deposit & Apply Tokens Confidentially");
        println!("4️⃣  Transfer Confidential Tokens");
        println!("5️⃣  Withdraw Confidential Tokens");
        println!("6️⃣  🚪 Exit");

        let mut option = String::new();

        stdin().read_line(&mut option).expect("❌ Invalid Input");
        let option: i8 = option.trim().parse().expect("❌ Invalid Input");

        match option {
            1 => loop {
                // For checking Token Accounts of Alice and Bob
                println!("👤 Check Token Account for:");
                println!("1️⃣  Alice");
                println!("2️⃣  Bob");
                let mut user = String::new();

                stdin().read_line(&mut user).expect("❌ Invalid Input");
                let user: i8 = user.trim().parse().expect("❌ Invalid Input");

                match user {
                    1 => {
                        println!("🔍 Fetching Token Account Details for Alice...");
                        fetch_token_account_with_extensions(
                            &rpc_client,
                            &alice_res.token_account_kp.pubkey(),
                        )
                        .await?;
                        break;
                    }
                    2 => {
                        println!("🔍 Fetching Token Account Details for Bob...");
                        fetch_token_account_with_extensions(
                            &rpc_client,
                            &bob_res.token_account_kp.pubkey(),
                        )
                        .await?;
                        break;
                    }
                    _ => {
                        println!("❌ Invalid selection");
                        break;
                    }
                }
            },
            2 => loop {
                println!("👤 Mint tokens for:");
                println!("1️⃣  Alice");
                println!("2️⃣  Bob");
                let mut user = String::new();

                stdin().read_line(&mut user).expect("❌ Invalid Input");
                let user: i8 = user.trim().parse().expect("❌ Invalid Input");

                match user {
                    1 => {
                        println!("💸 Enter amount to mint for Alice:");
                        let mut amount = String::new();
                        stdin().read_line(&mut amount).expect("❌ Invalid input");

                        let amount: u64 = amount.trim().parse().expect("❌ Invalid input");
                        token
                            .mint_to(
                                &alice_res.token_account_kp.pubkey(), // Destination
                                &alice.pubkey(),                      // Token Account authority
                                amount * 10u64.pow(6),                // Minting tokens
                                &[&alice],                            // Signers
                            )
                            .await?;

                        println!(
                            "✅ Successfully minted {} tokens for Alice!",
                            amount
                        );
                        break;
                    }
                    2 => {
                        println!("💸 Enter amount to mint for Bob:");
                        let mut amount = String::new();
                        stdin().read_line(&mut amount).expect("❌ Invalid Input");

                        let amount: u64 = amount.trim().parse().expect("❌ Invalid Input");
                        token
                            .mint_to(
                                &bob_res.token_account_kp.pubkey(), // Destination
                                &bob.pubkey(),                      // Token Account authority
                                amount * 10u64.pow(6),              // Minting tokens
                                &[&bob],                            // Signers
                            )
                            .await?;

                        println!(
                            "✅ Successfully minted {} tokens for Bob!",
                            amount
                        );
                        break;
                    }
                    _ => {
                        println!("🚫 No tokens minted.");
                        break;
                    }
                }
            },
            3 => {
                println!("👤 Deposit confidential tokens for:");
                println!("1️⃣  Alice");
                println!("2️⃣  Bob");
                let mut user = String::new();

                stdin().read_line(&mut user).expect("❌ Invalid Input");
                let user: i8 = user.trim().parse().expect("❌ Invalid Input");

                println!("💰 Enter amount to deposit confidentially:");
                let mut amount = String::new();
                stdin().read_line(&mut amount).expect("❌ Invalid input");

                let amount: u64 = amount.trim().parse().expect("❌ Invalid input");

                match user {
                    1 => {
                        // Depositing tokens for Alice's pending account and apply pending account to available balance
                        deposite_token_to_confidential(
                            &alice_res.token_account_kp,
                            &alice,
                            &token,
                            &alice_res.user_elgamal_kp,
                            &alice_res.user_aes_kp,
                            amount,
                        )
                        .await?;
                        println!("✅ Deposited {} tokens confidentially for Alice.", amount);
                    }
                    2 => {
                        // Depositing tokens for Bob's pending account and apply pending account to available balance
                        deposite_token_to_confidential(
                            &bob_res.token_account_kp,
                            &bob,
                            &token,
                            &bob_res.user_elgamal_kp,
                            &bob_res.user_aes_kp,
                            amount,
                        )
                        .await?;
                        println!("✅ Deposited {} tokens confidentially for Bob.", amount);
                    }
                    _ => {
                        println!("❌ Invalid selection");
                    }
                }
            }
            4 => {
                println!("👤 Transfer confidential tokens from:");
                println!("1️⃣  Alice");
                println!("2️⃣  Bob");
                let mut user = String::new();

                stdin().read_line(&mut user).expect("❌ Invalid Input");
                let user: i8 = user.trim().parse().expect("❌ Invalid Input");

                println!(
                    "🔄 Enter amount to transfer confidentially:"
                );
                let mut amount = String::new();
                stdin().read_line(&mut amount).expect("❌ Invalid input");

                let amount: u64 = amount.trim().parse().expect("❌ Invalid input");

                match user {
                    1 => {
                        // Transfer Tokens Confidentially Alice to Bob
                        println!("🔄 Transferring {} tokens confidentially from Alice to Bob...", amount);
                        transfer_tokens(
                            amount,
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
                        println!("✅ Transfer complete!");
                    }
                    2 => {
                        // Transfer Tokens Confidentially Bob to Alice
                        println!("🔄 Transferring {} tokens confidentially from Bob to Alice...", amount);
                        transfer_tokens(
                            amount,
                            &token,
                            &bob_res.token_account_kp,
                            &bob_res.user_elgamal_kp,
                            &bob_res.user_aes_kp,
                            &bob,
                            &alice,
                            &alice_res.user_elgamal_kp,
                            &alice_res.user_aes_kp,
                            &alice_res.token_account_kp,
                        )
                        .await?;
                        println!("✅ Transfer complete!");
                    }
                    _ => {
                        println!("❌ Invalid selection");
                    }
                }
            }
            5 => {
                println!("👤 Withdraw confidential tokens for:");
                println!("1️⃣  Alice");
                println!("2️⃣  Bob");
                let mut user = String::new();

                stdin().read_line(&mut user).expect("❌ Invalid Input");
                let user: i8 = user.trim().parse().expect("❌ Invalid Input");

                println!(
                    "🏧 Enter amount to withdraw confidentially:"
                );
                let mut amount = String::new();
                stdin().read_line(&mut amount).expect("❌ Invalid input");

                let amount: u64 = amount.trim().parse().expect("❌ Invalid input");

                match user {
                    1 => {
                        withdraw_tokens(
                            &alice_res.token_account_kp.pubkey(),
                            &alice_res.user_elgamal_kp,
                            &alice_res.user_aes_kp,
                            amount,
                            &token,
                            &alice,
                        )
                        .await?;
                        println!("✅ Withdrawn {} tokens confidentially for Alice.", amount);
                    }
                    2 => {
                        withdraw_tokens(
                            &bob_res.token_account_kp.pubkey(),
                            &bob_res.user_elgamal_kp,
                            &bob_res.user_aes_kp,
                            amount,
                            &token,
                            &bob,
                        )
                        .await?;
                        println!("✅ Withdrawn {} tokens confidentially for Bob.", amount);
                    }
                    _ => {
                        println!("❌ Invalid selection");
                    }
                }
            }
            6 => {
                println!("👋 Exiting. Goodbye!");
                break;
            }
            _ => {
                println!("❌ Invalid option. Please try again.");
            }
        }
    }

    Ok(())
}
