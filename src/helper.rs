use anyhow::{anyhow, Ok, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};
use spl_token_client::{
    client::{ProgramRpcClientSendTransaction, RpcClientResponse},
    spl_token_2022::{
        extension::{
            confidential_transfer::ConfidentialTransferMint, BaseStateWithExtensions,
            StateWithExtensionsOwned,
        },
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
        state::{Account, Mint},
    },
    token::Token,
};

// =================== Structs ===================

/// Holds the confidential token account keypair and associated cryptographic keys.
pub struct ConfTokenAccountRes {
    pub token_account_kp: Keypair,       // Token account keypair
    pub user_elgamal_kp: ElGamalKeypair, // ElGamal keypair for confidential encryption
    pub user_aes_kp: AeKey,              // AE key for confidential encryption
}

// =================== Helper Functions ===================

/// Generates a new keypair and funds it with 1 SOL from the faucet.
pub async fn keypair_gen(client: &RpcClient) -> Result<Keypair> {
    let keypair = Keypair::new();

    let sig = client
        .request_airdrop(&keypair.pubkey(), LAMPORTS_PER_SOL)
        .await?;

    client.confirm_transaction(&sig).await?;

    // Wait for confirmation
    loop {
        let is_confirmed = client.confirm_transaction(&sig).await?;
        if is_confirmed {
            break;
        }
    }

    Ok(keypair)
}

/// Fetches and prints the ConfidentialTransferMint extension for a mint account.
pub async fn fetch_mint_account(
    pub_key: &Pubkey,
    rpc_client: &Token<ProgramRpcClientSendTransaction>,
) -> Result<()> {
    let account = rpc_client.get_account(*pub_key).await?;

    let state: StateWithExtensionsOwned<Mint> = StateWithExtensionsOwned::unpack(account.data)?;

    let confirm = state.get_extension::<ConfidentialTransferMint>()?;
    println!("is auto approve:  {:?}", confirm.auto_approve_new_accounts);

    Ok(())
}

/// Submits a vector of instructions as a transaction and waits for confirmation.
pub async fn complete_ixs(
    rpc_client: &RpcClient,
    ix: Vec<Instruction>,
    signers: &[&Keypair],
    payer: &Keypair,
) -> Result<()> {
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    let trx =
        Transaction::new_signed_with_payer(&ix, Some(&payer.pubkey()), signers, recent_blockhash);

    let trx_sig = rpc_client.send_and_confirm_transaction(&trx).await?;

    println!("The Trx is successfully completed {}", trx_sig);

    Ok(())
}

/// Handles and prints the response from a token client transaction.
pub async fn handle_token_response(sig: &RpcClientResponse, content: String) -> Result<()> {
    match sig {
        RpcClientResponse::Simulation(rpc_res) => {
            if let Some(logs) = rpc_res.logs.clone() {
                for log in logs {
                    println!("The Log: {}", log);
                }
            }
        }
        RpcClientResponse::Signature(sig) => {
            println!("Sig for {} is: {}", content, sig.to_string());
        }
        _ => {}
    };

    Ok(())
}

/// Fetches and prints the confidential token account and its extensions.
pub async fn fetch_token_account_with_extensions(
    rpc_client: &RpcClient,
    token_account_pubkey: &Pubkey,
) -> Result<()> {
    // Fetch raw account data from the chain &[u8] type data
    let account_data = rpc_client
        .get_account_data(token_account_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch account data: {e}"))?;

    // Unpack with extensions
    let state_with_ext: StateWithExtensionsOwned<Account> =
        StateWithExtensionsOwned::unpack(account_data)
            .map_err(|e| anyhow!("Failed to unpack account with extensions: {e}"))?;

    // Print the base account data
    println!("\n Base Account: {:#?}", state_with_ext.base);

    // Find and print the ConfidentialTransfer extension if present
    // let ext = state_with_ext.get_extension::<ConfidentialTransferAccount>()?;

    // println!("\n Confidential Token Account {:#?}", ext);

    Ok(())
}
