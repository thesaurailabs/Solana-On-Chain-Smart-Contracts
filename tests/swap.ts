import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddress,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { BN } from "bn.js";

describe("swap", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Swap as Program<any>;
  const wallet = provider.wallet as anchor.Wallet;
  const buyer = Keypair.generate();

  let tokenMint: PublicKey;
  let adminTokenAccount: PublicKey;
  let vault: PublicKey;
  let vaultTokenAccount: PublicKey;
  let userTokenAccount: PublicKey;
  const index = new BN(Date.now()); // Use this index for PDA derivation

  before(async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(wallet.publicKey, 1_000_000_000)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(buyer.publicKey, 1_000_000_000)
    );

    tokenMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      null,
      6,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    adminTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      wallet.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      tokenMint,
      wallet.publicKey,
      undefined,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    await mintTo(
      provider.connection,
      wallet.payer,
      tokenMint,
      adminTokenAccount,
      wallet.publicKey,
      1_000_000,
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
  });

  it("Initializes the vault", async () => {
    const indexBuffer = Buffer.alloc(8);
    index.toArrayLike(Buffer, "le", 8).copy(indexBuffer);

    const [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), tokenMint.toBuffer(), indexBuffer],
      program.programId
    );
    vault = vaultPda;
    vaultTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      vault,
      true,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    await program.methods
      .initialize(index, new BN(1000))
      .accounts({
        authority: wallet.publicKey,
        vault: vault,
        tokenMint: tokenMint,
        vaultTokenAccount: vaultTokenAccount,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([wallet.payer])
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    console.log("Vault initialized:", vaultAccount);
  });

  it("Deposits tokens into the vault", async () => {
    await program.methods
      .depositTokens(new BN(500_000))
      .accounts({
        authority: wallet.publicKey,
        vault: vault,
        tokenMint: tokenMint,
        adminTokenAccount: adminTokenAccount,
        vaultTokenAccount: vaultTokenAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([wallet.payer])
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    console.log("Tokens deposited. Vault state:", vaultAccount);
  });

  it("Updates the token price", async () => {
    await program.methods
      .updatePrice(new BN(2000))
      .accounts({
        authority: wallet.publicKey,
        vault: vault,
      })
      .signers([wallet.payer])
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    console.log("Vault price after update:", vaultAccount.pricePerToken.toString());
  });

  it("Withdraws tokens from the vault to the admin", async () => {
  // Assume vault is already initialized and tokens have been deposited

     // Get admin token balance before withdrawal
     const adminBefore = await getAccount(
       provider.connection,
       adminTokenAccount,
       undefined,
       TOKEN_2022_PROGRAM_ID
     );
     const vaultBefore = await getAccount(
       provider.connection,
       vaultTokenAccount,
       undefined,
       TOKEN_2022_PROGRAM_ID
     );
   
     // Amount to withdraw (less than or equal to vault balance)
     const withdrawAmount = new BN(100_000);
   
     // Derive indexBuffer for PDA seeds (if needed)
     const indexBuffer = Buffer.alloc(8);
     index.toArrayLike(Buffer, "le", 8).copy(indexBuffer);
   
     // Derive vaultSigner PDA
     const [vaultSigner] = PublicKey.findProgramAddressSync(
       [Buffer.from("vault"), tokenMint.toBuffer(), indexBuffer],
       program.programId
     );
   
     // Call the withdraw_tokens instruction
     await program.methods
       .withdrawTokens(withdrawAmount)
       .accounts({
         authority: wallet.publicKey,
         vault: vault,
         tokenMint: tokenMint,
         adminTokenAccount: adminTokenAccount,
         vaultTokenAccount: vaultTokenAccount,
         vaultSigner: vaultSigner,
         tokenProgram: TOKEN_2022_PROGRAM_ID,
       })
       .signers([wallet.payer])
       .rpc();
   
     // Get balances after withdrawal
     const adminAfter = await getAccount(
       provider.connection,
       adminTokenAccount,
       undefined,
       TOKEN_2022_PROGRAM_ID
     );
     const vaultAfter = await getAccount(
       provider.connection,
       vaultTokenAccount,
       undefined,
       TOKEN_2022_PROGRAM_ID
     );
   
     // Assert balances
     console.log("Admin token balance before:", adminBefore.amount.toString());
     console.log("Admin token balance after:", adminAfter.amount.toString());
     console.log("Vault token balance before:", vaultBefore.amount.toString());
     console.log("Vault token balance after:", vaultAfter.amount.toString());
   
     if (
  adminAfter.amount !== adminBefore.amount + BigInt(withdrawAmount.toString())
) {
  throw new Error("Admin did not receive withdrawn tokens");
}
if (
  vaultAfter.amount !== vaultBefore.amount - BigInt(withdrawAmount.toString())
) {
  throw new Error("Vault did not send correct amount");
}
   });

  it("User purchases tokens", async () => {
    userTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      buyer.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    await createAssociatedTokenAccount(
      provider.connection,
      buyer,
      tokenMint,
      buyer.publicKey,
      undefined,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const indexBuffer = Buffer.alloc(8);
    index.toArrayLike(Buffer, "le", 8).copy(indexBuffer);

    const [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), tokenMint.toBuffer(), indexBuffer],
      program.programId
    );
    const solUsdPriceAccount = new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");

    await program.methods
      .purchaseTokens(new BN(10_000))
      .accounts({
        buyer: buyer.publicKey,
        admin: wallet.publicKey,
        vault: vaultPda,
        tokenMint: tokenMint,
        vaultTokenAccount: vaultTokenAccount,
        userTokenAccount: userTokenAccount,
        vaultSigner: vaultPda,
        solUsdPrice: solUsdPriceAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vaultPda);
    console.log("User purchased tokens. Vault state:", vaultAccount);
  });


it("Closes the vault and admin receives remaining tokens after expiry", async () => {
        const indexBuffer = Buffer.alloc(8);

    const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), tokenMint.toBuffer(), indexBuffer],
    program.programId
  );
  const vaultTokenAccount = await getAssociatedTokenAddress(
    tokenMint,
    vaultPda,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  const adminBefore = await getAccount(
    provider.connection,
    adminTokenAccount,
    undefined,
    TOKEN_2022_PROGRAM_ID
  );
  const vaultBefore = await getAccount(
    provider.connection,
    vaultTokenAccount,
    undefined,
    TOKEN_2022_PROGRAM_ID
  );

  // Close vault (now expired)
  await program.methods
    .closeVault()
    .accounts({
      vault: vaultPda,
      owner: wallet.publicKey,
      vaultTokenAccount: vaultTokenAccount,
      adminTokenAccount: adminTokenAccount,
      tokenMint: tokenMint,
      vaultSigner: vaultPda,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .signers([wallet.payer])
    .rpc();

  // Check balances after closing
  const adminAfter = await getAccount(
    provider.connection,
    adminTokenAccount,
    undefined,
    TOKEN_2022_PROGRAM_ID
  );
 let vaultTokenAccountClosed = false;
   let vaultAfterBalance = undefined;

try {
  await getAccount(
    provider.connection,
    vaultTokenAccount,
    undefined,
    TOKEN_2022_PROGRAM_ID
  );
} catch (e) {
  // For spl-token >=0.3.9, the error name is "TokenAccountNotFoundError"
  // Sometimes it may not have a name property, so check the message too
  if (e.name === "TokenAccountNotFoundError" || e.message?.includes("TokenAccountNotFoundError")) {
    vaultTokenAccountClosed = true; // Expected: account was closed
    vaultAfterBalance = 0n;
  } else {
    throw e; // Unexpected error, rethrow
  }
}
if (!vaultTokenAccountClosed) {
  throw new Error("Vault token account was not closed");
}


  console.log("Admin token balance before:", adminBefore.amount.toString());
  console.log("Admin token balance after:", adminAfter.amount.toString());
  console.log("Vault token balance before:", vaultBefore.amount.toString());
  console.log("Vault token balance after:", vaultAfterBalance.toString());

  if (adminBefore.amount + vaultBefore.amount !== adminAfter.amount) {
    throw new Error("Admin did not receive all tokens from vault");
  }
  if (vaultAfterBalance !== 0n) {
    throw new Error("Vault was not emptied");
  }
  if (!vaultAccountClosed) {
    throw new Error("Vault account was not closed");
  }
});
});