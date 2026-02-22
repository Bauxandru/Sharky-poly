/**
 * Order Matcher
 *
 * Polls the indexer API for matchable order pairs and submits
 * match_orders transactions to Solana.
 *
 * A pair is matchable when:
 *   best_lender_apr_bps ≤ best_borrower_max_apr_bps
 *   AND their principal_lamports are equal
 */

import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import axios from "axios";
import * as fs from "fs";
import * as path from "path";
import { createLogger } from "./logger";

const logger = createLogger("matcher");

export interface OrderBookInfo {
  pubkey: string;
  collection_mint: string;
  duration_seconds: number;
  best_lender_apr_bps: number;
  best_borrower_max_apr_bps: number;
  open_lender_orders: number;
  open_borrower_orders: number;
}

export interface LenderOrder {
  pubkey: string;
  lender: string;
  principal_lamports: number;
  apr_bps: number;
  nonce: number;
}

export interface BorrowerOrder {
  pubkey: string;
  borrower: string;
  nft_mint: string;
  principal_lamports: number;
  max_apr_bps: number;
}

export class Matcher {
  private connection: Connection;
  private crankKeypair: Keypair;
  private apiUrl: string;

  // Program ID
  private programId = new PublicKey("625a3ioDN2HBSGZwpRFKGWcAVSc2XbySBRubDcMKxVjo");

  constructor(apiUrl: string, rpcEndpoint: string, keypairPath: string) {
    this.apiUrl = apiUrl;
    this.connection = new Connection(rpcEndpoint, "confirmed");
    this.crankKeypair = loadKeypair(keypairPath);
    logger.info(`Crank wallet: ${this.crankKeypair.publicKey.toBase58()}`);
  }

  async runMatchingCycle(): Promise<number> {
    // 1. Fetch all orderbooks that have potential matches
    const orderbooks = await this.fetchMatchableOrderbooks();
    if (orderbooks.length === 0) return 0;

    let matched = 0;

    for (const ob of orderbooks) {
      try {
        const count = await this.matchOrderbook(ob);
        matched += count;
      } catch (err) {
        logger.warn(`Failed to match orderbook ${ob.pubkey}: ${err}`);
      }
    }

    return matched;
  }

  private async fetchMatchableOrderbooks(): Promise<OrderBookInfo[]> {
    const { data } = await axios.get<OrderBookInfo[]>(`${this.apiUrl}/api/orderbooks`);
    // Filter to only those with potential matches
    return data.filter(
      (ob) =>
        ob.best_lender_apr_bps > 0 &&
        ob.best_borrower_max_apr_bps > 0 &&
        ob.best_lender_apr_bps <= ob.best_borrower_max_apr_bps &&
        ob.open_lender_orders > 0 &&
        ob.open_borrower_orders > 0
    );
  }

  private async matchOrderbook(ob: OrderBookInfo): Promise<number> {
    // Fetch best lender and borrower orders
    const [lenderOrders, borrowerOrders] = await Promise.all([
      this.fetchLenderOrders(ob.pubkey),
      this.fetchBorrowerOrders(ob.pubkey),
    ]);

    if (lenderOrders.length === 0 || borrowerOrders.length === 0) return 0;

    let matched = 0;
    let li = 0; // index into sorted lender orders (sorted ASC by APR)
    let bi = 0; // index into sorted borrower orders (sorted DESC by max APR)

    // Greedy matching: pair best lender with best borrower
    while (li < lenderOrders.length && bi < borrowerOrders.length) {
      const lender = lenderOrders[li];
      const borrower = borrowerOrders[bi];

      if (lender.apr_bps > borrower.max_apr_bps) {
        // No more matches possible (both lists exhausted from best)
        break;
      }

      if (lender.principal_lamports !== borrower.principal_lamports) {
        // Amounts don't match — skip to next borrower
        bi++;
        continue;
      }

      // Submit match transaction
      try {
        const sig = await this.submitMatch(ob, lender, borrower);
        logger.info(
          `Matched: lender=${lender.lender.slice(0, 8)} @ ${lender.apr_bps}bps ` +
            `<-> borrower=${borrower.borrower.slice(0, 8)} @ ${borrower.max_apr_bps}bps | sig=${sig}`
        );
        matched++;
        li++;
        bi++;
      } catch (err) {
        logger.warn(`Match tx failed: ${err}`);
        li++; // Skip this lender order and try next
      }
    }

    return matched;
  }

  private async fetchLenderOrders(orderbook: string): Promise<LenderOrder[]> {
    const { data } = await axios.get<LenderOrder[]>(
      `${this.apiUrl}/api/orderbooks/${orderbook}/lender-orders?status=open&limit=50`
    );
    return data; // Already sorted ASC by apr_bps
  }

  private async fetchBorrowerOrders(orderbook: string): Promise<BorrowerOrder[]> {
    const { data } = await axios.get<BorrowerOrder[]>(
      `${this.apiUrl}/api/orderbooks/${orderbook}/borrower-orders?status=open&limit=50`
    );
    return data; // Already sorted DESC by max_apr_bps
  }

  private async submitMatch(
    ob: OrderBookInfo,
    lender: LenderOrder,
    borrower: BorrowerOrder
  ): Promise<string> {
    const orderbookPk = new PublicKey(ob.pubkey);
    const lenderOrderPk = new PublicKey(lender.pubkey);
    const borrowerOrderPk = new PublicKey(borrower.pubkey);
    const lenderPk = new PublicKey(lender.lender);
    const borrowerPk = new PublicKey(borrower.borrower);

    // Derive loan PDA
    const [loanPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("loan"), lenderOrderPk.toBuffer(), borrowerOrderPk.toBuffer()],
      this.programId
    );

    // Build match_orders instruction
    // In production, use the generated Anchor client/IDL for this
    const ix = new TransactionInstruction({
      programId: this.programId,
      keys: [
        { pubkey: orderbookPk, isSigner: false, isWritable: true },
        { pubkey: this.getGlobalStatePda(), isSigner: false, isWritable: true },
        { pubkey: lenderOrderPk, isSigner: false, isWritable: true },
        { pubkey: borrowerOrderPk, isSigner: false, isWritable: true },
        { pubkey: loanPda, isSigner: false, isWritable: true },
        { pubkey: lenderPk, isSigner: false, isWritable: true },   // lender_refund
        { pubkey: borrowerPk, isSigner: false, isWritable: true },  // borrower_wallet
        { pubkey: borrowerPk, isSigner: false, isWritable: true },  // borrower_refund
        { pubkey: this.crankKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: this.encodeMatchOrdersIx(),
    });

    const tx = new Transaction();
    // Add priority fee to ensure timely inclusion
    tx.add(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 5_000 }));
    tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }));
    tx.add(ix);

    tx.feePayer = this.crankKeypair.publicKey;
    tx.recentBlockhash = (await this.connection.getLatestBlockhash()).blockhash;
    tx.sign(this.crankKeypair);

    const sig = await this.connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: false,
      maxRetries: 3,
    });

    await this.connection.confirmTransaction(sig, "confirmed");
    return sig;
  }

  private getGlobalStatePda(): PublicKey {
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_state")],
      this.programId
    );
    return pda;
  }

  private encodeMatchOrdersIx(): Buffer {
    // Anchor instruction discriminator: sha256("global:match_orders")[0..8]
    const discriminator = Buffer.from([17, 1, 201, 93, 7, 51, 251, 134]);
    return discriminator;
  }
}

function loadKeypair(keypairPath: string): Keypair {
  const expanded = keypairPath.replace("~", process.env.HOME ?? "");
  const raw = JSON.parse(fs.readFileSync(expanded, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}
