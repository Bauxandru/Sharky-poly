/**
 * NFT Lending Crank Bot
 *
 * Responsibilities:
 * 1. Match compatible lender/borrower orders on-chain
 * 2. Update best APR fields on orderbooks after cancellations
 * 3. Submit maker scores to on-chain for reward epoch tracking
 *
 * Cost strategy: reads matchable pairs from the indexer DB (via API),
 * only submits Solana transactions when there's something to do.
 * This minimizes RPC write costs.
 */

import * as dotenv from "dotenv";
import { Matcher } from "./matcher";
import { RewardScoreSubmitter } from "./rewards";
import { createLogger } from "./logger";

dotenv.config();

const logger = createLogger("crank");

const API_URL = process.env.API_URL ?? "http://localhost:8080";
const RPC_ENDPOINT = process.env.RPC_ENDPOINT!;
const CRANK_KEYPAIR_PATH = process.env.CRANK_KEYPAIR_PATH ?? "~/.config/solana/id.json";

if (!RPC_ENDPOINT) {
  logger.error("RPC_ENDPOINT must be set");
  process.exit(1);
}

async function main() {
  logger.info("Starting NFT Lending Crank Bot");
  logger.info(`API: ${API_URL}`);
  logger.info(`RPC: ${RPC_ENDPOINT}`);

  const matcher = new Matcher(API_URL, RPC_ENDPOINT, CRANK_KEYPAIR_PATH);
  const scorer = new RewardScoreSubmitter(API_URL, RPC_ENDPOINT, CRANK_KEYPAIR_PATH);

  // Run matching loop every 5 seconds
  const matchingLoop = async () => {
    while (true) {
      try {
        const matched = await matcher.runMatchingCycle();
        if (matched > 0) {
          logger.info(`Matched ${matched} order pairs`);
        }
      } catch (err) {
        logger.error("Matching cycle error:", err);
      }
      await sleep(5_000);
    }
  };

  // Submit scores every 60 seconds
  const scoringLoop = async () => {
    while (true) {
      await sleep(60_000);
      try {
        await scorer.submitPendingScores();
      } catch (err) {
        logger.error("Scoring cycle error:", err);
      }
    }
  };

  // Run both loops concurrently
  await Promise.all([matchingLoop(), scoringLoop()]);
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

main().catch((err) => {
  logger.error("Fatal error:", err);
  process.exit(1);
});
