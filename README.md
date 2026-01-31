# The SaurAI Labs Solana Vesting & Presale Programs

This repository contains two secure, Anchor-based Solana programs designed for token management and distribution:

1.  **Vesting Program**: A robust system for linear vesting schedules with configurable cliff periods.
2.  **Presale (Swap) Program**: A token sale contract utilizing Pyth Oracles for real-time SOL/USD pricing.

## Features

### 🔐 SaurAI Vesting Program (`programs/vesting`)
Designed to manage long-term token release schedules for team members, investors, or community rewards.

*   **Custom Schedules**: Configurable start time, cliff duration, and monthly intervals.
*   **Cliff Protection**: Tokens remain locked until the cliff period expires.
*   **Linear Release**: Tokens unlock monthly after the cliff.
*   **Admin Controls**: Secure creation and funding of reserves.
*   **Safety Checks**: Built-in verification to prevent premature closing of accounts with remaining funds.
*   **Test Coverage**: Comprehensive tests covering full lifecycle and edge cases (e.g., closing attempts with remaining balance).

### 💱 SaurAI Presale(Swap) Program (`programs/swap`)
A "Vault" based system for selling tokens at a fixed USD price, accepting SOL payments.

*   **Oracle Integration**: Uses [Pyth Network](https://pyth.network/) for accurate, real-time SOL/USD price feeds.
*   **Dynamic Pricing**: Calculates SOL amount required based on the fixed USD token price.
*   **Purchase Limits**: Enforces a maximum token limit per transaction (1M tokens).
*   **Vault Management**: Admin functions to deposit/withdraw inventory and update prices.
*   **Security**: Admin-only access for critical vault operations.

## Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [Solana CLI](https://docs.solanalabs.com/cli/install)
*   [Anchor CLI](https://www.anchor-lang.com/docs/installation)
*   Node.js & Yarn

## Getting Started

1.  **Install Dependencies**
    ```bash
    yarn install
    ```

2.  **Build Programs**
    ```bash
    anchor build
    ```

3.  **Run Tests**
    The repo includes comprehensive typescript tests for the vesting logic.
    ```bash
    anchor test
    ```
    *Note: Ensure you have a local validator running or are configured for a testnet.*

## Deployment

For detailed, step-by-step deployment instructions—including keypair generation, IDL publication, and Solscan verification—please refer to the **[Deployment Guide](./deployment_guide.md)**.

## Project Structure

*   `programs/test`: Source code for the Vesting Program.
*   `programs/swap`: Source code for the Presale/Swap Program.
*   `tests/`: TypeScript integration tests to verify program logic.
*   `deployment_guide.md`: Chronological guide for mainnet/devnet operations.

## Security

*   **Audits**: Smart contracts contain security.txt fields pointing to contact/policy info.
*   **Access Control**: Critical instructions (Create Reserve, Update Price, Withdraw) are restricted to the hardcoded Admin Authority key.

---
*Built with [Anchor Framework](https://www.anchor-lang.com/)*
