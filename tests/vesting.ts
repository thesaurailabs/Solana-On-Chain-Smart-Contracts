// import * as anchor from "@coral-xyz/anchor";
// import { Program } from "@coral-xyz/anchor";
// import { PublicKey, Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY, Transaction } from "@solana/web3.js";
// import {
//   TOKEN_2022_PROGRAM_ID,
//   ASSOCIATED_TOKEN_PROGRAM_ID,
//   createInitializeMintInstruction,
//   MINT_SIZE,
//   getAssociatedTokenAddress,
//   createAssociatedTokenAccountInstruction,
//   createMintToInstruction,
//   getAccount,
// } from "@solana/spl-token";
// import { BN } from "bn.js";
// import {
//   startAnchor,
//   Clock,
//   BanksClient,
//   ProgramTestContext,
// } from "solana-bankrun";
// import { BankrunProvider } from "anchor-bankrun";

// async function getBankrunClockUnixTimestamp(provider: anchor.AnchorProvider) {
//   const clockAccount = await provider.connection.getAccountInfo(SYSVAR_CLOCK_PUBKEY);
//   if (!clockAccount || !clockAccount.data) throw new Error("Clock sysvar missing");
//   // The unixTimestamp is at offset 32 as per Solana's clock sysvar layout
//   return clockAccount.data.readBigInt64LE(32);
// }

// describe("vesting", () => {
//   let context: ProgramTestContext;
//   let provider: any;
//   let program: Program<any>;
//   let wallet: anchor.Wallet;
//   // Explicitly use a custom payer
//   const payer = Keypair.generate();
//   const beneficiary = Keypair.generate();

//   let tokenMint: PublicKey;
//   let ownerTokenAccount: PublicKey;
//   let treasuryTokenAccount: PublicKey;
//   let beneficiaryTokenAccount: PublicKey;
//   let vestingAccount: PublicKey;
//   let reserveAccount: PublicKey;
//   const reserveType = "counsil59";

//   let startTime: number;
//   let cliffTime: number;
//   let endTime: number;
//   const totalAmount = 100_000;
//   const monthlyClaim = 20_000;

//  before(async () => {
//   context = await startAnchor(".", [], [
//     {
//       address: payer.publicKey,
//       keypair: payer,
//       info: { lamports: 10_000_000_000, data: Buffer.alloc(0), owner: SystemProgram.programId, executable: false },
//     },
//     {
//       address: beneficiary.publicKey,
//       keypair: beneficiary,
//       info: { lamports: 1_000_000_000, data: Buffer.alloc(0), owner: SystemProgram.programId, executable: false },
//     },
//   ]);
//   provider = new BankrunProvider(context);
//   anchor.setProvider(provider);
//   program = anchor.workspace.Vesting as Program<any>;
//   wallet = provider.wallet as anchor.Wallet;

//   // Use the bankrun clock for test timestamps
//   const now = await getBankrunClockUnixTimestamp(provider);
//   const secondsInMonth = 30 * 24 * 60 * 60; // 2,592,000 seconds

//   startTime = Number(now);
//   cliffTime = startTime + secondsInMonth; // Cliff is 1 month after start
//   endTime = startTime + 6 * secondsInMonth; // End is 6 months after start

//   // --- MANUAL MINT CREATION ---
//   const mintKeypair = Keypair.generate();
//   tokenMint = mintKeypair.publicKey;

//   // Calculate rent for mint
//   const mintRent = await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE);

//   // Create and initialize mint account
//   let tx = new Transaction();
//   tx.add(
//     SystemProgram.createAccount({
//       fromPubkey: payer.publicKey,
//       newAccountPubkey: tokenMint,
//       space: MINT_SIZE,
//       lamports: mintRent,
//       programId: TOKEN_2022_PROGRAM_ID,
//     }),
//     createInitializeMintInstruction(
//       tokenMint,
//       6,
//       payer.publicKey,
//       null,
//       TOKEN_2022_PROGRAM_ID
//     )
//   );
//   await provider.sendAndConfirm(tx, [payer, mintKeypair]);

//   // --- MANUAL ASSOCIATED TOKEN ACCOUNT CREATION ---
//   ownerTokenAccount = await getAssociatedTokenAddress(
//     tokenMint,
//     payer.publicKey,
//     false,
//     TOKEN_2022_PROGRAM_ID,
//     ASSOCIATED_TOKEN_PROGRAM_ID
//   );
//   tx = new Transaction();
//   tx.add(
//     createAssociatedTokenAccountInstruction(
//       payer.publicKey,
//       ownerTokenAccount,
//       payer.publicKey,
//       tokenMint,
//       TOKEN_2022_PROGRAM_ID,
//       ASSOCIATED_TOKEN_PROGRAM_ID
//     )
//   );
//   await provider.sendAndConfirm(tx, [payer]);

//   // --- MANUAL MINT TO ---
//   tx = new Transaction();
//   tx.add(
//     createMintToInstruction(
//       tokenMint,
//       ownerTokenAccount,
//       payer.publicKey,
//       totalAmount,
//       [],
//       TOKEN_2022_PROGRAM_ID
//     )
//   );
//   await provider.sendAndConfirm(tx, [payer]);
// });

//   it("Creates vesting account", async () => {
//     const [vestingPda] = PublicKey.findProgramAddressSync(
//       [Buffer.from(reserveType)],
//       program.programId
//     );
//     vestingAccount = vestingPda;

//     treasuryTokenAccount = await getAssociatedTokenAddress(
//       tokenMint,
//       vestingAccount,
//       true,
//       TOKEN_2022_PROGRAM_ID,
//       ASSOCIATED_TOKEN_PROGRAM_ID
//     );

//     await program.methods
//       .createVestingAccount(reserveType)
//       .accounts({
//         signer: payer.publicKey,
//         vestingAccount: vestingAccount,
//         mint: tokenMint,
//         treasuryTokenAccount: treasuryTokenAccount,
//         tokenProgram: TOKEN_2022_PROGRAM_ID,
//         systemProgram: SystemProgram.programId,
//         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
//       })
//       .signers([payer])
//       .rpc();

//     const vesting = await program.account.vestingAccount.fetch(vestingAccount);
//     console.log("Vesting account created:", vesting);
//   });

//   it("Creates reserve and transfers tokens to treasury", async () => {
//     const [reservePda] = PublicKey.findProgramAddressSync(
//       [Buffer.from("reserve"), vestingAccount.toBuffer()],
//       program.programId
//     );
//     reserveAccount = reservePda;

//     await program.methods
//       .createReserve(
//         new BN(startTime),
//         new BN(endTime),
//         new BN(totalAmount),
//         new BN(cliffTime),
//         new BN(monthlyClaim),
//       )
//       .accounts({
//         owner: payer.publicKey,
//         beneficiary: beneficiary.publicKey,
//         ownerTokenAccount: ownerTokenAccount,
//         vestingAccount: vestingAccount,
//         reserveAccount: reserveAccount,
//         treasuryTokenAccount: treasuryTokenAccount,
//         mint: tokenMint,
//         tokenProgram: TOKEN_2022_PROGRAM_ID,
//         systemProgram: SystemProgram.programId,
//       })
//       .signers([payer])
//       .rpc();

//     const reserve = await program.account.reserveAccount.fetch(reserveAccount);
//     console.log("Reserve account created:", reserve);
//   });

// it("Claims tokens after cliff, once per month for 6 months", async () => {
//   const secondsInMonth = 30 * 24 * 60 * 60; // 2,592,000 seconds

//   // Print reserve state for debugging
//   const reserve = await program.account.reserveAccount.fetch(reserveAccount);
//   const vStartTime = reserve.startTime.toNumber() + reserve.cliffTime.toNumber();

//   beneficiaryTokenAccount = await getAssociatedTokenAddress(
//     tokenMint,
//     beneficiary.publicKey,
//     false,
//     TOKEN_2022_PROGRAM_ID,
//     ASSOCIATED_TOKEN_PROGRAM_ID
//   );
//   let tx = new Transaction();
//   tx.add(
//     createAssociatedTokenAccountInstruction(
//       beneficiary.publicKey,
//       beneficiaryTokenAccount,
//       beneficiary.publicKey,
//       tokenMint,
//       TOKEN_2022_PROGRAM_ID,
//       ASSOCIATED_TOKEN_PROGRAM_ID
//     )
//   );
//   await provider.sendAndConfirm(tx, [beneficiary]);

//   // First claim: immediately after reserve creation (should fail with CliffPeriodNotEnded)
//   const clockBeforeFirstClaim = await context.banksClient.getClock();
//   console.log("Before first claim - unixTimestamp:", clockBeforeFirstClaim.unixTimestamp);

//   try {
//     await program.methods
//       .claimTokens(reserveType)
//       .accounts({
//         beneficiary: beneficiary.publicKey,
//         reserveAccount: reserveAccount,
//         vestingAccount: vestingAccount,
//         mint: tokenMint,
//         treasuryTokenAccount: treasuryTokenAccount,
//         beneficiaryTokenAccount: beneficiaryTokenAccount,
//         tokenProgram: TOKEN_2022_PROGRAM_ID,
//         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
//         systemProgram: SystemProgram.programId,
//       })
//       .signers([beneficiary])
//       .rpc();
//     throw new Error("Expected CliffPeriodNotEnded error, got success");
//   } catch (err: any) {
//     if (!err.toString().includes("CliffPeriodNotEnded")) {
//       throw new Error("Expected CliffPeriodNotEnded error, got: " + err.toString());
//     }
//     console.log("Month 0: Correctly failed due to cliff period not ended");
//   }

//   // Set clock to just after v_start_time + 1st period
//   const claimStartTime = vStartTime + secondsInMonth;

//   const currentClock = await context.banksClient.getClock();
//   const newClock = new Clock(
//     currentClock.slot,
//     currentClock.epochStartTimestamp,
//     currentClock.epoch,
//     currentClock.leaderScheduleEpoch,
//     BigInt(claimStartTime)
//   );
//   console.log("Setting clock to claimStartTime:", claimStartTime);
//   await context.setClock(newClock);

//   const clockAfterCliff = await context.banksClient.getClock();
//   console.log("After setting cliff - unixTimestamp:", clockAfterCliff.unixTimestamp);

//   const numClaims = 6;
//   for (let i = 0; i < numClaims-1; i++) {
//     const clockBeforeClaim = await context.banksClient.getClock();
//     console.log(`Month ${i + 1} - Before claim - unixTimestamp:`, clockBeforeClaim.unixTimestamp);

//     await program.methods
//       .claimTokens(reserveType)
//       .accounts({
//         beneficiary: beneficiary.publicKey,
//         reserveAccount: reserveAccount,
//         vestingAccount: vestingAccount,
//         mint: tokenMint,
//         treasuryTokenAccount: treasuryTokenAccount,
//         beneficiaryTokenAccount: beneficiaryTokenAccount,
//         tokenProgram: TOKEN_2022_PROGRAM_ID,
//         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
//         systemProgram: SystemProgram.programId,
//       })
//       .signers([beneficiary])
//       .rpc();

//     const beneficiaryAccount = await getAccount(
//       provider.connection,
//       beneficiaryTokenAccount,
//       undefined,
//       TOKEN_2022_PROGRAM_ID
//     );
//     console.log(
//       `Month ${i + 1}: Claimed. Beneficiary token balance: ${beneficiaryAccount.amount.toString()}`
//     );

//     // Advance time by 1 month for next claim
//     const nextClaimTime = vStartTime + (i + 2) * secondsInMonth;
//     const nextClock = new Clock(
//       clockBeforeClaim.slot,
//       clockBeforeClaim.epochStartTimestamp,
//       clockBeforeClaim.epoch,
//       clockBeforeClaim.leaderScheduleEpoch,
//       BigInt(nextClaimTime)
//     );
//     console.log(`Month ${i + 1} - Advancing clock to unixTimestamp:`, nextClaimTime);
//     await context.setClock(nextClock);

//     const clockAfterSet = await context.banksClient.getClock();
//     console.log(`Month ${i + 1} - After setClock - unixTimestamp:`, clockAfterSet.unixTimestamp);

    
//   }
//  const reserve1 = await program.account.reserveAccount.fetch(reserveAccount);

// console.log("Reserve account state:");
// console.log({
//   beneficiary: reserve1.beneficiary.toBase58(),
//   startTime: reserve1.startTime.toNumber(),
//   endTime: reserve1.endTime.toNumber(),
//   totalAmount: reserve1.totalAmount.toNumber(),
//   amountWithdrawn: reserve1.amountWithdrawn.toNumber(),
//   cliffTime: reserve1.cliffTime.toNumber(),
//   monthlyClaim: reserve1.monthlyClaim.toNumber(),
//   vestingAccount: reserve1.vestingAccount.toBase58(),
//   bump: reserve1.bump,
// });

// });
// it("Closes the reserve account", async () => {
//   // Call the close_reserve_account instruction
//   await program.methods
//     .closeReserveAccount()
//     .accounts({
//       reserveAccount: reserveAccount,
//       vestingAccount: vestingAccount,
//       beneficiary: beneficiary.publicKey,
//       systemProgram: SystemProgram.programId,
//     })
//     .signers([beneficiary])
//     .rpc();

//   // After closing, trying to fetch the reserve account should fail
//   let closed = false;
//   try {
//     await program.account.reserveAccount.fetch(reserveAccount);
//   } catch (err) {
//     closed = true;
//     console.log("Reserve account is closed as expected.");
//   }
//   if (!closed) throw new Error("Reserve account was not closed!");
// });
// });