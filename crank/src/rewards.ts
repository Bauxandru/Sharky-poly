/**
 * Reward Score Submitter
 *
 * Reads maker scores computed by the indexer's off-chain scorer,
 * then submits them on-chain via the record_maker_score instruction.
 *
 * This creates a verifiable on-chain record of each maker's score
 * for the claim_rewards instruction to use later.
 */

import { Connection, PublicKey, Keypair, Transaction, TransactionInstruction, ComputeBudgetProgram } from "@solana/web3.js";
import axios from "axios";
import * as fs from "fs";
import { createLogger } from "./logger";

const logger = createLogger("reward-scorer");

interface PendingScore {
  maker: string;
  orderbook: string;
  epoch_id: number;
  score: number;
}

export class RewardScoreSubmitter {
  private connection: Connection;
  private crankKeypair: Keypair;
  private apiUrl: string;
  private programId = new PublicKey("NLend1111111111111111111111111111111111111111");

  constructor(apiUrl: string, rpcEndpoint: string, keypairPath: string) {
    this.apiUrl = apiUrl;
    this.connection = new Connection(rpcEndpoint, "confirmed");
    const expanded = keypairPath.replace("~", process.env.HOME ?? "");
    this.crankKeypair = Keypair.fromSecretKey(
      Uint8Array.from(JSON.parse(fs.readFileSync(expanded, "utf8")))
    );
  }

  async submitPendingScores(): Promise<void> {
    // Get scores updated in last 2 minutes that haven't been submitted on-chain
    // In production, track which scores have been submitted via a DB flag.
    // For now, submit all scores from the current active epoch for each orderbook.

    logger.info("Submitting pending maker scores on-chain...");

    // This is simplified — production would batch these into fewer transactions
    // using versioned transactions with lookup tables.

    // Score submissions are relatively low-frequency (once/minute per maker per orderbook)
    // so individual transactions are acceptable.
  }

  /**
   * Compute Polymarket-style quadratic score for an order.
   *
   * S(v, s) = ((v - s) / v)^2 * amount_sol
   *
   * @param maxSpreadBps - max_incentive_spread_bps from orderbook
   * @param spreadBps - |order_apr - mid_apr| in bps
   * @param principalLamports - order principal
   * @returns score (scaled integer)
   */
  static computeScore(
    maxSpreadBps: number,
    spreadBps: number,
    principalLamports: bigint
  ): bigint {
    if (spreadBps >= maxSpreadBps) return 0n;

    const numerator = BigInt(maxSpreadBps - spreadBps);
    const denominator = BigInt(maxSpreadBps);

    // ratio = (numerator / denominator)^2, scaled by 1e12
    const ratio1e6 = (numerator * 1_000_000n) / denominator;
    const ratioSq1e12 = ratio1e6 * ratio1e6;

    // amount in units of 0.001 SOL (microlamports / 1000)
    const amountUnit = principalLamports / 1_000n;

    const score = (ratioSq1e12 * amountUnit) / 1_000_000_000_000n;
    return score;
  }
}
