import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DaoVoting } from "../target/types/dao_voting";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  createMint,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";

const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
);

describe("dao-voting", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.DaoVoting as Program<DaoVoting>;

  const admin = Keypair.generate();
  const user = Keypair.generate();
  const partnership = Keypair.generate();

  let globalState: PublicKey;
  let governanceTokenMint: PublicKey;
  let stGovernanceTokenMint: PublicKey;
  let vault: PublicKey;
  let voteTokenMint: PublicKey;
  let userGovernanceTokenAccount: PublicKey;
  let userStGovernanceTokenAccount: PublicKey;
  let userVoteTokenAccount: PublicKey;
  let projectTokenMint: PublicKey;
  let partnershipProjectTokenAccount: PublicKey;
  let vaultTokenAccount: PublicKey;
  let userProjectTokenAccount: PublicKey;
  let userVault: PublicKey;
  let metadataAddress: PublicKey;
  let stMetadataAddress: PublicKey;
  let voteTokenMetadataAddress: PublicKey;

  const vaultId = new anchor.BN(1);

  before(async () => {
    // Airdrop SOL to admin, user, and partnership
    for (const account of [admin, user, partnership]) {
      const signature = await provider.connection.requestAirdrop(
        account.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(signature);
    }

    // Find PDA addresses
    [globalState] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_state")],
      program.programId,
    );

    [governanceTokenMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("governance_token_mint")],
      program.programId,
    );

    [stGovernanceTokenMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("st_governance_token_mint")],
      program.programId,
    );

    [vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), vaultId.toBuffer("le", 8)],
      program.programId,
    );

    [voteTokenMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("vote_token_mint"), vault.toBuffer()],
      program.programId,
    );

    [metadataAddress] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        governanceTokenMint.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID,
    );

    [stMetadataAddress] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        stGovernanceTokenMint.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID,
    );
  });

  it("Initializes the DAO", async () => {
    const metadata = {
      name: "Governance Token",
      symbol: "GOV",
      uri: "https://example.com/metadata.json",
    };

    const stMetadata = {
      name: "ST Governance Token",
      symbol: "STGOV",
      uri: "https://example.com/st-metadata.json",
    };

    try {
      await program.methods
        .initialize(metadata, stMetadata)
        .accounts({
          globalState,
          governanceTokenMint,
          stGovernanceTokenMint,
          metadata: metadataAddress,
          stMetadata: stMetadataAddress,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([admin])
        .rpc();

      const globalStateAccount = await program.account.globalState.fetch(
        globalState,
      );
      expect(globalStateAccount.admin.toString()).to.equal(
        admin.publicKey.toString(),
      );
      expect(globalStateAccount.governanceTokenMint.toString()).to.equal(
        governanceTokenMint.toString(),
      );
      expect(globalStateAccount.stGovernanceTokenMint.toString()).to.equal(
        stGovernanceTokenMint.toString(),
      );
    } catch (error) {
      console.error("Error in initialize:", error);
      throw error;
    }
  });

  it("Converts governance tokens to ST governance tokens", async () => {
    const amount = new anchor.BN(500);

    // 创建用户的 governance token 账户
    userGovernanceTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      governanceTokenMint,
      user.publicKey,
    ).then((account) => account.address);

    // 铸造一些 governance tokens 给用户
    await mintTo(
      provider.connection,
      admin,
      governanceTokenMint,
      userGovernanceTokenAccount,
      admin,
      1000,
    );

    // 创建用户的 ST governance token 账户
    userStGovernanceTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      stGovernanceTokenMint,
      user.publicKey,
    ).then((account) => account.address);

    try {
      await program.methods
        .convertToStGovernance(amount)
        .accounts({
          governanceTokenMint,
          stGovernanceTokenMint,
          userGovernanceTokenAccount,
          userStGovernanceTokenAccount,
          user: user.publicKey,
          globalState,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([user])
        .rpc();

      const userGovernanceTokenBalance =
        await provider.connection.getTokenAccountBalance(
          userGovernanceTokenAccount,
        );
      expect(userGovernanceTokenBalance.value.amount).to.equal("500");

      const userStGovernanceTokenBalance =
        await provider.connection.getTokenAccountBalance(
          userStGovernanceTokenAccount,
        );
      expect(userStGovernanceTokenBalance.value.amount).to.equal("500");
    } catch (error) {
      console.error("Error in convertToStGovernance:", error);
      throw error;
    }
  });

  it("Creates stA and vault", async () => {
    const maxVoteCap = new anchor.BN(1000000); // Example max vote cap
    const deadline = new anchor.BN(Math.floor(Date.now() / 1000) + 10); // 1 hour from now
    const metadata = {
      name: "Vote Token",
      symbol: "VOTE",
      uri: "https://example.com/vote-metadata.json",
    };

    [voteTokenMetadataAddress] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        voteTokenMint.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID,
    );

    try {
      await program.methods
        .createStAAndVault(vaultId, maxVoteCap, deadline, metadata)
        .accounts({
          vault,
          voteTokenMint,
          globalState,
          admin: admin.publicKey,
          metadata: voteTokenMetadataAddress,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([admin])
        .rpc();
    } catch (error) {
      console.error("Error in createStAAndVault:", error);
      throw error;
    }

    const vaultAccount = await program.account.vault.fetch(vault);
    expect(vaultAccount.owner.toString()).to.equal(admin.publicKey.toString());
    expect(vaultAccount.governanceTokenMint.toString()).to.equal(
      governanceTokenMint.toString(),
    );
    expect(vaultAccount.stGovernanceTokenMint.toString()).to.equal(
      stGovernanceTokenMint.toString(),
    );
    expect(vaultAccount.voteTokenMint.toString()).to.equal(
      voteTokenMint.toString(),
    );
    expect(vaultAccount.vaultId.toString()).to.equal(vaultId.toString());
    expect(vaultAccount.maxVoteCap.toString()).to.equal(maxVoteCap.toString());
    expect(vaultAccount.deadline.toString()).to.equal(deadline.toString());
  });

  it("Votes", async () => {
    const amount = new anchor.BN(100);

    [userVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_vault"), vault.toBuffer(), user.publicKey.toBuffer()],
      program.programId,
    );

    userVoteTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      voteTokenMint,
      user.publicKey,
    ).then((account) => account.address);

    try {
      await program.methods
        .vote(vaultId, amount)
        .accounts({
          stGovernanceTokenMint,
          voteTokenMint,
          userStGovernanceTokenAccount,
          userVoteTokenAccount,
          vault,
          userVault,
          user: user.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();

      const vaultAccount = await program.account.vault.fetch(vault);
      expect(vaultAccount.totalBurned.toString()).to.equal("100");

      const userVaultAccount = await program.account.userVault.fetch(userVault);
      expect(userVaultAccount.burnedAmount.toString()).to.equal("100");

      const userStGovernanceTokenBalance =
        await provider.connection.getTokenAccountBalance(
          userStGovernanceTokenAccount,
        );
      expect(userStGovernanceTokenBalance.value.amount).to.equal("400");
    } catch (error) {
      console.error("Error in vote:", error);
      throw error;
    }
  });

  it("Fails to vote after deadline", async () => {
    // Wait for the deadline to pass
    await new Promise((resolve) => setTimeout(resolve, 10 * 1000));

    const amount = new anchor.BN(50);

    try {
      await program.methods
        .vote(vaultId, amount)
        .accounts({
          stGovernanceTokenMint,
          voteTokenMint,
          userStGovernanceTokenAccount,
          userVoteTokenAccount,
          vault,
          userVault,
          user: user.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();

      // If we reach here, the test should fail
      expect.fail("Expected an error, but none was thrown");
    } catch (error) {
      expect(error.message).to.include("Voting period has ended");
    }
  });

  it("Sets project token", async () => {
    projectTokenMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6,
    );

    const convertTime = Math.floor(Date.now() / 1000) + 10; // 30 seconds from now

    await program.methods
      .setProjectToken(projectTokenMint, new anchor.BN(convertTime))
      .accounts({
        vault,
        owner: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    expect(vaultAccount.projectTokenMint.toString()).to.equal(
      projectTokenMint.toString(),
    );
    expect(vaultAccount.convertTime.toString()).to.equal(
      convertTime.toString(),
    );
  });

  it("Deposits project tokens", async () => {
    const amount = new anchor.BN(1000);

    // Create partnership project token account and mint some tokens
    partnershipProjectTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      projectTokenMint,
      partnership.publicKey,
    ).then((account) => account.address);

    await mintTo(
      provider.connection,
      admin,
      projectTokenMint,
      partnershipProjectTokenAccount,
      admin,
      2000,
    );

    vaultTokenAccount = await getAssociatedTokenAddressSync(
      projectTokenMint,
      vault,
      true,
    );

    await program.methods
      .depositProjectTokens(amount)
      .accounts({
        vault,
        projectTokenAccount: partnershipProjectTokenAccount,
        vaultTokenAccount,
        projectTokenMint,
        projectAuthority: partnership.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([partnership])
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    expect(vaultAccount.totalDeposited.toString()).to.equal("1000");
  });

  it("Claims project tokens", async () => {
    await new Promise((resolve) => setTimeout(resolve, 10 * 1000)); // Wait for 30 seconds

    userProjectTokenAccount = await getAssociatedTokenAddressSync(
      projectTokenMint,
      user.publicKey,
    );

    await program.methods
      .claimProjectTokens(vaultId)
      .accounts({
        vault,
        voteTokenMint,
        projectTokenMint,
        userVoteTokenAccount,
        vaultTokenAccount,
        userProjectTokenAccount,
        userVault,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const userVaultAccount = await program.account.userVault.fetch(userVault);
    expect(userVaultAccount.burnedAmount.toString()).to.equal("0");

    const userProjectTokenAccountInfo =
      await provider.connection.getTokenAccountBalance(userProjectTokenAccount);
    expect(userProjectTokenAccountInfo.value.amount).to.equal("1000");
  });
});
