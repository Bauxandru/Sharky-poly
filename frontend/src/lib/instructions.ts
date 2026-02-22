/**
 * Build Solana instructions for the NFT Lending program.
 *
 * In production: generate this from the Anchor IDL using `anchor build`
 * and then `anchor idl fetch <program_id>`. Here we build manually
 * until the program is deployed and the IDL is available.
 */

import {
  PublicKey,
  TransactionInstruction,
  SystemProgram,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

export const PROGRAM_ID = new PublicKey(
  process.env.NEXT_PUBLIC_PROGRAM_ID ??
    "625a3ioDN2HBSGZwpRFKGWcAVSc2XbySBRubDcMKxVjo"
);

export function getGlobalStatePda(): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    PROGRAM_ID
  );
  return pda;
}

export function getLenderOrderPda(
  orderbook: PublicKey,
  lender: PublicKey,
  nonce: bigint
): PublicKey {
  const nonceBytes = Buffer.alloc(8);
  nonceBytes.writeBigUInt64LE(nonce);
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("lender_order"), orderbook.toBuffer(), lender.toBuffer(), nonceBytes],
    PROGRAM_ID
  );
  return pda;
}

export function getBorrowerOrderPda(
  orderbook: PublicKey,
  nftMint: PublicKey
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("borrower_order"), orderbook.toBuffer(), nftMint.toBuffer()],
    PROGRAM_ID
  );
  return pda;
}

export function getProgramAuthorityPda(): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("program_authority")],
    PROGRAM_ID
  );
  return pda;
}

/**
 * Build place_lender_order instruction.
 * Uses Anchor's 8-byte discriminator: sha256("global:place_lender_order")[0..8]
 */
export function buildLenderOrderIx(
  lender: PublicKey,
  orderbook: PublicKey,
  principalLamports: bigint,
  aprBps: bigint,
  expiryTs: bigint
): TransactionInstruction {
  // Instruction data: discriminator + args (borsh-encoded)
  const disc = anchorDiscriminator("global:place_lender_order");
  const data = Buffer.alloc(8 + 8 + 8 + 8);
  disc.copy(data, 0);
  data.writeBigUInt64LE(principalLamports, 8);
  data.writeBigUInt64LE(aprBps, 16);
  data.writeBigInt64LE(expiryTs, 24);

  // Derive order PDA (nonce=0 simplified; real impl reads nonce from orderbook)
  const orderPda = getLenderOrderPda(orderbook, lender, 0n);

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: orderbook, isSigner: false, isWritable: true },
      { pubkey: orderPda, isSigner: false, isWritable: true },
      { pubkey: lender, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/**
 * Build place_borrower_order instruction.
 */
export function buildBorrowerOrderIx(
  borrower: PublicKey,
  orderbook: PublicKey,
  nftMint: PublicKey,
  principalLamports: bigint,
  maxAprBps: bigint,
  expiryTs: bigint
): TransactionInstruction {
  const disc = anchorDiscriminator("global:place_borrower_order");
  const data = Buffer.alloc(8 + 8 + 8 + 8);
  disc.copy(data, 0);
  data.writeBigUInt64LE(principalLamports, 8);
  data.writeBigUInt64LE(maxAprBps, 16);
  data.writeBigInt64LE(expiryTs, 24);

  const orderPda = getBorrowerOrderPda(orderbook, nftMint);
  const programAuthority = getProgramAuthorityPda();

  // Derive borrower's token account for the NFT
  // (simplified; real impl uses associated token account lookup)
  const nftTokenAccount = PublicKey.findProgramAddressSync(
    [borrower.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), nftMint.toBuffer()],
    new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe8bXh")
  )[0];

  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: orderbook, isSigner: false, isWritable: true },
      { pubkey: orderPda, isSigner: false, isWritable: true },
      { pubkey: nftMint, isSigner: false, isWritable: false },
      { pubkey: nftTokenAccount, isSigner: false, isWritable: true },
      // metadata account derived off-chain
      { pubkey: borrower, isSigner: true, isWritable: true },
      { pubkey: programAuthority, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

/** Anchor instruction discriminators: sha256("global:<name>")[0..8] */
const DISCRIMINATORS: Record<string, number[]> = {
  "global:place_lender_order":    [145, 245, 114,  59, 121,  44, 162,  60],
  "global:cancel_lender_order":   [ 63, 204,  25,  27, 217, 111, 199,  75],
  "global:place_borrower_order":  [ 74, 124, 218, 199, 163,  46,  53,  61],
  "global:cancel_borrower_order": [ 30,  75, 182, 179, 147, 116, 222, 183],
  "global:match_orders":          [ 17,   1, 201,  93,   7,  51, 251, 134],
  "global:repay_loan":            [224,  93, 144,  77,  61,  17, 137,  54],
  "global:foreclose_loan":        [203,  84, 119, 226, 137,   1, 180,  23],
  "global:record_maker_score":    [248, 206, 253, 115, 105,  81, 240,  60],
  "global:claim_rewards":         [  4, 144, 132,  71, 116,  23, 151,  80],
};

/** Return the Anchor discriminator for a named instruction */
function anchorDiscriminator(name: string): Buffer {
  const disc = DISCRIMINATORS[name];
  if (!disc) throw new Error(`Unknown discriminator: ${name}`);
  return Buffer.from(disc);
}
