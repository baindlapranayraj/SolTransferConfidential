# Confidential Solana Token Example

This project demonstrates how to use Solana's SPL Token 2022 program with the Confidential Transfer extension, enabling private token transfers using zero-knowledge proofs. The code is written in Rust and uses the Solana and SPL Token client libraries.

## Features
- **Create a confidential mint**: Deploy a new SPL Token mint with the Confidential Transfer extension enabled.
- **Create confidential token accounts**: Set up token accounts for users (e.g., Alice and Bob) with confidential transfer capabilities.
- **Mint tokens**: Mint tokens to a confidential token account.
- **Deposit tokens confidentially**: Move tokens into a confidential (private) balance using zero-knowledge proofs.
- **Confidential transfer**: Privately transfer tokens between accounts without revealing the amount on-chain.
- **Apply pending balances**: Move deposited tokens from a pending state to an available confidential balance.

## How It Works
1. **Setup**: Connects to a local Solana RPC node and generates keypairs for users.
2. **Mint Creation**: Creates a new token mint with confidential transfer enabled.
3. **Account Creation**: Sets up confidential token accounts for each user, generating the necessary cryptographic keys.
4. **Minting**: Mints tokens to a user's confidential token account.
5. **Deposit**: Deposits tokens into the confidential balance (pending, then available).
6. **Confidential Transfer**: Transfers tokens privately from one user to another using zero-knowledge proofs.

## Confidential Transfer Flow
<p align="center">
  <img src="./images/confTransferFlow.png" alt="Confidential Transfer Flow" width="600"/>
</p>

## Confidential Pending and Available Accounts
When you deposit tokens into a confidential token account, the tokens first go into a **pending balance**. This means the tokens are not immediately available for spending—they are waiting to be confirmed and applied. After the deposit, you must perform an additional step to "apply" the pending balance, which moves the tokens into the **available balance**. 

Only tokens in the available balance can be used for confidential transfers or withdrawals. This two-step process helps ensure privacy and security in confidential token operations.

## Confidential Token Account Cycle
<p align="center">
  <img src="./images/confTACycle.png" alt="Confidential Token Account Cycle" width="600"/>
</p>
This cycle allows for seamless movement between public and confidential states, supporting privacy-preserving transactions while maintaining compatibility with public token systems. Each stage is crucial for ensuring both privacy and flexibility in managing digital assets.


## File Structure
- `src/main.rs`: Main entry point. Orchestrates the confidential mint, account creation, minting, deposit, and transfer steps.
- `src/helper.rs`: Helper functions for keypair generation, transaction handling, and account inspection.
- `src/confidential/`: Contains modules for each confidential token operation:
  - `confidential_mint.rs`: Create a confidential mint.
  - `confidential_token_account.rs`: Create confidential token accounts.
  - `confidential_deposit_token.rs`: Deposit tokens confidentially.
  - `confidential_transfer_tokens.rs`: Confidential token transfer logic.
  - `apply_pending_balance.rs`: Apply pending confidential balances.
  - `confidential_withdraw_tokens.rs`: (Stub for future withdrawal logic.)

## Prerequisites
- Rust toolchain
- Solana CLI and local validator running (`solana-test-validator`)
- The required Solana and SPL Token 2022 Rust crates

## Running the Example
1. Start a local Solana validator:
   ```sh
   solana-test-validator
   ```
2. Build and run the project:
   ```sh
   cd confidential-solana
   cargo run
   ```

You should see logs for each step: mint creation, account setup, minting, deposit, and confidential transfer.

## Problems I Have Faced

| Problem                                                                 | Reason                                                                                      | Solution                                                                                                         |
|-------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------|
| Unable to find proper reference for building confidential program using Anchor and no JS SDK for testing | Lack of documentation and SDK support for confidential programs with Anchor and JS           | After spending a couple of days, started building from the client side using the `spl-token-2022` crate          |
| Unable to create confidential token account                             | `spl-token-2022` is not supported by the default local validator                            | Manually installed the mainnet validator and then started creating confidential token accounts                   |


## Notes
- This example is for educational purposes and uses insecure keypair generation for demonstration.
- The confidential transfer feature relies on zero-knowledge proofs and special cryptographic keys (ElGamal, AES).
- Withdraw logic is stubbed and can be implemented similarly to deposit/apply logic.

## References
- [Solana Confidential Transfer Helius Blog](https://www.helius.dev/blog/confidential-balances)
- [Solana Confidential Transfer Quciknode Blog](https://www.quicknode.com/guides/solana-development/spl-tokens/token-2022/confidential#:~:text=The%20Confidential%20Transfer%20extension%20enables,tokens%20without%20revealing%20the%20amounts)
- [Solana Confidential Transfers](https://github.com/solana-foundation/solana-com/blob/main/content/docs/en/tokens/extensions/confidential-transfer/index.mdx)

---

Feel free to explore the code and experiment with confidential token operations on your local Solana network!
