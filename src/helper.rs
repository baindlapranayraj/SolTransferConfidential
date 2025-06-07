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
            confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
            BaseStateWithExtensions, StateWithExtensionsOwned,
        },
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
        state::{Account, Mint},
    },
    token::Token,
};

// =================== Structs ===================

pub struct ConfTokenAccountRes {
    pub token_account_kp: Keypair,
    pub user_elgamal_kp: ElGamalKeypair,
    pub user_aes_kp: AeKey,
}

// =================== Helper Functions ===================

// Generats Keypair and fund some SOL's init
pub async fn keypair_gen(client: &RpcClient) -> Result<Keypair> {
    let keypair = Keypair::new();

    let sig = client
        .request_airdrop(&keypair.pubkey(), LAMPORTS_PER_SOL)
        .await?;

    client.confirm_transaction(&sig).await?;

    loop {
        let is_confirmed = client.confirm_transaction(&sig).await?;
        if is_confirmed {
            break;
        }
    }

    Ok(keypair)
}

// Fetches ConfidentialTransferMint Account
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

pub async fn fetch_token_account_with_extensions(
    rpc_client: &RpcClient,
    token_account_pubkey: &Pubkey,
) -> Result<()> {
    // Fetch raw account data from the chain
    let account_data = rpc_client
        .get_account_data(token_account_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch account data: {e}"))?;

    // Unpack with extensions
    let state_with_ext: StateWithExtensionsOwned<Account> =
        StateWithExtensionsOwned::unpack(account_data)
            .map_err(|e| anyhow!("Failed to unpack account with extensions: {e}"))?;

    // Print the base account data
    println!("Base Account: {:#?}", state_with_ext.base);

    // Find and print the ConfidentialTransfer extension if present
    let ext = state_with_ext.get_extension::<ConfidentialTransferAccount>()?;

    println!("Confidential Elgamal Pubkey {:#?}", ext);

    Ok(())
}
