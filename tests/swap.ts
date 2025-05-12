import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Swap } from "../target/types/swap";
import {
  SystemProgram,
  PublicKey,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  getAccount,
  createAssociatedTokenAccountIdempotent,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

describe("swap", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.swap as Program<Swap>;
  const payer = provider.wallet;

  const [programStatePda, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from("program-state")],
    program.programId
  );

  // ✅ Substitua aqui pelo seu Mint LBX correto
  const lbxMint = new PublicKey("9yU4EX7KKtbR5dQr8RAfKRoDR9Ro4k7urcmJkCqDL5cy");
  const vault = new PublicKey("5ArPQSA9vM7sukJzsFdkEnUzG5NALCDDcEm6Li5VoZRS");

  const solAmount = new anchor.BN(10 * LAMPORTS_PER_SOL);
  const userAta = getAssociatedTokenAddressSync(
    lbxMint,
    payer.publicKey,
    false
  );

  it("Executa swap de 10 SOL por LBX", async () => {
    // 🔧 Criar ATA se necessário
    try {
      await getAccount(provider.connection, userAta);
      console.log("✅ ATA já existe.");
    } catch (_) {
      console.log("🔧 Criando ATA...");
      await createAssociatedTokenAccountIdempotent(
        provider.connection,
        payer.payer,
        lbxMint,
        payer.publicKey
      );
      console.log("✅ ATA criado com sucesso.");
    }

    const ataBefore = await getAccount(provider.connection, userAta);

    const tx = await program.methods
      .swap(solAmount)
      .accounts({
        user: payer.publicKey,
        userLbxAta: userAta,
        lbxMint,
        programState: programStatePda,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([])
      .rpc();

    const ataAfter = await getAccount(provider.connection, userAta);

    console.log("\n=== 🔁 SWAP EXECUTADO ===");
    console.log("🧾 TX:", tx);
    console.log("📤 SOL enviado:", solAmount.toNumber() / LAMPORTS_PER_SOL);
    console.log("🎯 LBX recebido:", ataAfter.amount.toString());
    console.log("📈 Diferença:", Number(ataAfter.amount) - Number(ataBefore.amount));
    console.log("🏦 Vault:", vault.toBase58());
    console.log("🪙 Mint LBX:", lbxMint.toBase58());
    console.log("📦 PDA:", programStatePda.toBase58());
  });
});
