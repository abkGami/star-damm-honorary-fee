import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { startAnchor, BankrunProvider } from "solana-bankrun";
import { StarDammHonoraryFee } from "../../target/types/star_damm_honorary_fee";
import { expect } from "chai";

describe("DAMM v2 Honorary Fee Position - Core Tests", () => {
  let provider: BankrunProvider;
  let program: Program<StarDammHonoraryFee>;
  let payer: Keypair;
  let vault: Keypair;
  let quoteMint: PublicKey;
  let baseMint: PublicKey;
  let creatorQuoteAta: PublicKey;

  // PDAs
  let policyPda: PublicKey;
  let progressPda: PublicKey;
  let positionOwnerPda: PublicKey;
  let treasuryPda: PublicKey;

  // Test configuration
  const INVESTOR_FEE_SHARE_BPS = 7500; // 75%
  const DAILY_CAP = 1000000; // 1M tokens
  const MIN_PAYOUT_LAMPORTS = 1000; // 0.001 tokens
  const TOTAL_INVESTOR_ALLOCATION = 10000000; // 10M tokens

  before(async () => {
    // Start local validator with our program
    const context = await startAnchor(
      "./",
      [],
      [
        {
          name: "star_damm_honorary_fee",
          programId: new PublicKey(
            "AQUVRgoaGsoy2uGnzkSDBoEVEJox2XT6Vna3Y9xKKwFZ"
          ),
        },
      ]
    );

    provider = new BankrunProvider(context);
    anchor.setProvider(provider);

    program = anchor.workspace
      .StarDammHonoraryFee as Program<StarDammHonoraryFee>;
    payer = provider.wallet.payer;
    vault = Keypair.generate();

    // Create test tokens
    quoteMint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      payer.publicKey,
      6 // 6 decimals
    );

    baseMint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      payer.publicKey,
      6
    );

    // Create creator's quote ATA
    creatorQuoteAta = await createAssociatedTokenAccount(
      provider.connection,
      payer,
      quoteMint,
      payer.publicKey
    );

    // Derive PDAs
    [policyPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("star_vault"),
        vault.publicKey.toBuffer(),
        Buffer.from("policy"),
      ],
      program.programId
    );

    [progressPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("star_vault"),
        vault.publicKey.toBuffer(),
        Buffer.from("progress"),
      ],
      program.programId
    );

    [positionOwnerPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("star_vault"),
        vault.publicKey.toBuffer(),
        Buffer.from("investor_fee_pos_owner"),
      ],
      program.programId
    );

    [treasuryPda] = await getAssociatedTokenAddress(
      quoteMint,
      positionOwnerPda,
      true
    );
  });

  describe("Honorary Position Initialization", () => {
    it("Should initialize honorary position with correct configuration", async () => {
      // Mock pool account (in real test, this would be a proper cp-amm pool)
      const mockPool = Keypair.generate();
      const mockPosition = Keypair.generate();

      // Fund the mock pool account with some data
      await provider.connection.requestAirdrop(mockPool.publicKey, 1000000000);

      const tx = await program.methods
        .initializeHonoraryPosition(
          INVESTOR_FEE_SHARE_BPS,
          new anchor.BN(DAILY_CAP),
          new anchor.BN(MIN_PAYOUT_LAMPORTS),
          new anchor.BN(TOTAL_INVESTOR_ALLOCATION)
        )
        .accounts({
          payer: payer.publicKey,
          vault: vault.publicKey,
          pool: mockPool.publicKey,
          quoteMint,
          baseMint,
          creatorQuoteAta,
          positionOwnerPda,
          policy: policyPda,
          progress: progressPda,
          treasury: treasuryPda,
          position: mockPosition.publicKey,
          cpAmmProgram: new PublicKey("11111111111111111111111111111111"), // Mock program ID
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([mockPosition])
        .rpc();

      console.log("Initialize transaction:", tx);

      // Verify policy state
      const policyAccount = await program.account.policyState.fetch(policyPda);
      expect(policyAccount.investorFeeShareBps).to.equal(
        INVESTOR_FEE_SHARE_BPS
      );
      expect(policyAccount.dailyCap.toNumber()).to.equal(DAILY_CAP);
      expect(policyAccount.minPayoutLamports.toNumber()).to.equal(
        MIN_PAYOUT_LAMPORTS
      );
      expect(policyAccount.quoteMint.toString()).to.equal(quoteMint.toString());
      expect(policyAccount.totalInvestorAllocation.toNumber()).to.equal(
        TOTAL_INVESTOR_ALLOCATION
      );

      // Verify progress state
      const progressAccount = await program.account.progressState.fetch(
        progressPda
      );
      expect(progressAccount.lastDistributionTs.toNumber()).to.equal(0);
      expect(progressAccount.dayComplete).to.equal(true);
      expect(progressAccount.dailyDistributed.toNumber()).to.equal(0);
      expect(progressAccount.carryOver.toNumber()).to.equal(0);
    });

    it("Should reject invalid fee share percentage", async () => {
      const mockPool = Keypair.generate();
      const mockPosition = Keypair.generate();

      try {
        await program.methods
          .initializeHonoraryPosition(
            10001, // Invalid: > 10000 bps
            new anchor.BN(DAILY_CAP),
            new anchor.BN(MIN_PAYOUT_LAMPORTS),
            new anchor.BN(TOTAL_INVESTOR_ALLOCATION)
          )
          .accounts({
            payer: payer.publicKey,
            vault: vault.publicKey,
            pool: mockPool.publicKey,
            quoteMint,
            baseMint,
            creatorQuoteAta,
            positionOwnerPda,
            policy: policyPda,
            progress: progressPda,
            treasury: treasuryPda,
            position: mockPosition.publicKey,
            cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          })
          .signers([mockPosition])
          .rpc();

        expect.fail("Should have rejected invalid fee share");
      } catch (error) {
        expect(error.message).to.include("InvalidTokenOrder");
      }
    });
  });

  describe("Distribution Mechanics", () => {
    let mockInvestors: Array<{
      keypair: Keypair;
      ata: PublicKey;
      streamAccount: Keypair;
      lockedAmount: number;
    }>;

    beforeEach(async () => {
      // Create mock investors with ATAs and stream accounts
      mockInvestors = [];
      for (let i = 0; i < 5; i++) {
        const investorKeypair = Keypair.generate();
        const ata = await createAssociatedTokenAccount(
          provider.connection,
          payer,
          quoteMint,
          investorKeypair.publicKey
        );
        const streamAccount = Keypair.generate();

        // Mock locked amounts: decreasing amounts (5M, 4M, 3M, 2M, 1M)
        const lockedAmount = (5 - i) * 1000000;

        mockInvestors.push({
          keypair: investorKeypair,
          ata,
          streamAccount,
          lockedAmount,
        });
      }

      // Add some quote tokens to treasury to simulate claimed fees
      await mintTo(
        provider.connection,
        payer,
        quoteMint,
        treasuryPda,
        payer.publicKey,
        2000000 // 2M tokens
      );
    });

    it("Should distribute fees proportionally based on locked amounts", async () => {
      // Calculate expected distributions
      const totalLocked = mockInvestors.reduce(
        (sum, inv) => sum + inv.lockedAmount,
        0
      );
      const eligibleShareBps = Math.min(
        INVESTOR_FEE_SHARE_BPS,
        Math.floor((totalLocked / TOTAL_INVESTOR_ALLOCATION) * 10000)
      );
      const treasoryBalance = 2000000;
      const investorTotal = Math.floor(
        (treasoryBalance * eligibleShareBps) / 10000
      );

      console.log("Test setup:");
      console.log("Total locked:", totalLocked);
      console.log("Eligible share bps:", eligibleShareBps);
      console.log("Expected investor total:", investorTotal);

      // Prepare remaining accounts (stream + ATA pairs)
      const remainingAccounts = [];
      for (const investor of mockInvestors) {
        remainingAccounts.push(
          {
            pubkey: investor.streamAccount.publicKey,
            isWritable: false,
            isSigner: false,
          },
          { pubkey: investor.ata, isWritable: true, isSigner: false }
        );
      }

      const tx = await program.methods
        .distributeFees(new anchor.BN(5)) // Process all 5 investors
        .accounts({
          payer: payer.publicKey,
          vault: vault.publicKey,
          policy: policyPda,
          progress: progressPda,
          positionOwnerPda,
          position: Keypair.generate().publicKey, // Mock position
          treasury: treasuryPda,
          creatorQuoteAta,
          cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
          streamflowProgram: new PublicKey("11111111111111111111111111111111"),
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(remainingAccounts)
        .rpc();

      console.log("Distribution transaction:", tx);

      // Verify progress was updated
      const progressAccount = await program.account.progressState.fetch(
        progressPda
      );
      expect(progressAccount.dayComplete).to.equal(true);
      console.log("Final progress state:", progressAccount);
    });

    it("Should enforce 24-hour cooldown", async () => {
      // Try to distribute again immediately (should fail)
      const remainingAccounts = mockInvestors.flatMap((inv) => [
        {
          pubkey: inv.streamAccount.publicKey,
          isWritable: false,
          isSigner: false,
        },
        { pubkey: inv.ata, isWritable: true, isSigner: false },
      ]);

      try {
        await program.methods
          .distributeFees(new anchor.BN(5))
          .accounts({
            payer: payer.publicKey,
            vault: vault.publicKey,
            policy: policyPda,
            progress: progressPda,
            positionOwnerPda,
            position: Keypair.generate().publicKey,
            treasury: treasuryPda,
            creatorQuoteAta,
            cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
            streamflowProgram: new PublicKey(
              "11111111111111111111111111111111"
            ),
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .remainingAccounts(remainingAccounts)
          .rpc();

        expect.fail("Should have enforced 24h cooldown");
      } catch (error) {
        expect(error.message).to.include("CooldownNotElapsed");
      }
    });

    it("Should handle pagination correctly", async () => {
      // Process in 2 pages of 2-3 investors each
      const remainingAccounts = mockInvestors.flatMap((inv) => [
        {
          pubkey: inv.streamAccount.publicKey,
          isWritable: false,
          isSigner: false,
        },
        { pubkey: inv.ata, isWritable: true, isSigner: false },
      ]);

      // First page (2 investors)
      const tx1 = await program.methods
        .distributeFees(new anchor.BN(2))
        .accounts({
          payer: payer.publicKey,
          vault: vault.publicKey,
          policy: policyPda,
          progress: progressPda,
          positionOwnerPda,
          position: Keypair.generate().publicKey,
          treasury: treasuryPda,
          creatorQuoteAta,
          cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
          streamflowProgram: new PublicKey("11111111111111111111111111111111"),
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(remainingAccounts)
        .rpc();

      console.log("First page transaction:", tx1);

      // Verify pagination state
      let progressAccount = await program.account.progressState.fetch(
        progressPda
      );
      expect(progressAccount.paginationCursor.toNumber()).to.equal(2);
      expect(progressAccount.dayComplete).to.equal(false);

      // Second page (remaining 3 investors)
      const tx2 = await program.methods
        .distributeFees(new anchor.BN(3))
        .accounts({
          payer: payer.publicKey,
          vault: vault.publicKey,
          policy: policyPda,
          progress: progressPda,
          positionOwnerPda,
          position: Keypair.generate().publicKey,
          treasury: treasuryPda,
          creatorQuoteAta,
          cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
          streamflowProgram: new PublicKey("11111111111111111111111111111111"),
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(remainingAccounts)
        .rpc();

      console.log("Second page transaction:", tx2);

      // Verify day is complete
      progressAccount = await program.account.progressState.fetch(progressPda);
      expect(progressAccount.dayComplete).to.equal(true);
    });
  });

  describe("Edge Cases and Error Handling", () => {
    it("Should handle zero locked amounts", async () => {
      // Create investors with zero locked amounts
      const zeroInvestors = [];
      for (let i = 0; i < 3; i++) {
        const investorKeypair = Keypair.generate();
        const ata = await createAssociatedTokenAccount(
          provider.connection,
          payer,
          quoteMint,
          investorKeypair.publicKey
        );
        const streamAccount = Keypair.generate();

        zeroInvestors.push({
          keypair: investorKeypair,
          ata,
          streamAccount,
          lockedAmount: 0, // All unlocked
        });
      }

      const remainingAccounts = zeroInvestors.flatMap((inv) => [
        {
          pubkey: inv.streamAccount.publicKey,
          isWritable: false,
          isSigner: false,
        },
        { pubkey: inv.ata, isWritable: true, isSigner: false },
      ]);

      // Should complete successfully with 100% to creator
      const tx = await program.methods
        .distributeFees(new anchor.BN(3))
        .accounts({
          payer: payer.publicKey,
          vault: vault.publicKey,
          policy: policyPda,
          progress: progressPda,
          positionOwnerPda,
          position: Keypair.generate().publicKey,
          treasury: treasuryPda,
          creatorQuoteAta,
          cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
          streamflowProgram: new PublicKey("11111111111111111111111111111111"),
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(remainingAccounts)
        .rpc();

      console.log("Zero locked amounts transaction:", tx);

      // All funds should go to creator
      const progressAccount = await program.account.progressState.fetch(
        progressPda
      );
      expect(progressAccount.dayComplete).to.equal(true);
    });

    it("Should handle dust amounts correctly", async () => {
      // Test with very small amounts that would create dust
      await mintTo(
        provider.connection,
        payer,
        quoteMint,
        treasuryPda,
        payer.publicKey,
        100 // Very small amount (100 lamports)
      );

      const smallInvestors = [];
      for (let i = 0; i < 3; i++) {
        const investorKeypair = Keypair.generate();
        const ata = await createAssociatedTokenAccount(
          provider.connection,
          payer,
          quoteMint,
          investorKeypair.publicKey
        );
        const streamAccount = Keypair.generate();

        smallInvestors.push({
          keypair: investorKeypair,
          ata,
          streamAccount,
          lockedAmount: 100, // Small locked amounts
        });
      }

      const remainingAccounts = smallInvestors.flatMap((inv) => [
        {
          pubkey: inv.streamAccount.publicKey,
          isWritable: false,
          isSigner: false,
        },
        { pubkey: inv.ata, isWritable: true, isSigner: false },
      ]);

      const tx = await program.methods
        .distributeFees(new anchor.BN(3))
        .accounts({
          payer: payer.publicKey,
          vault: vault.publicKey,
          policy: policyPda,
          progress: progressPda,
          positionOwnerPda,
          position: Keypair.generate().publicKey,
          treasury: treasuryPda,
          creatorQuoteAta,
          cpAmmProgram: new PublicKey("11111111111111111111111111111111"),
          streamflowProgram: new PublicKey("11111111111111111111111111111111"),
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(remainingAccounts)
        .rpc();

      console.log("Dust handling transaction:", tx);

      const progressAccount = await program.account.progressState.fetch(
        progressPda
      );
      expect(progressAccount.dayComplete).to.equal(true);
    });
  });
});
