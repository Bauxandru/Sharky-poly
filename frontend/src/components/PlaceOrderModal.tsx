"use client";

import { useState } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, Transaction, SystemProgram } from "@solana/web3.js";
import type { OrderBook } from "@/lib/types";
import { formatApr, formatDuration } from "@/lib/format";
import { buildLenderOrderIx, buildBorrowerOrderIx } from "@/lib/instructions";
import { X } from "lucide-react";

interface Props {
  orderbook: OrderBook;
  side: "lender" | "borrower";
  onClose: () => void;
}

export function PlaceOrderModal({ orderbook, side, onClose }: Props) {
  const { publicKey, sendTransaction } = useWallet();
  const { connection } = useConnection();

  const [principalSol, setPrincipalSol] = useState("");
  const [aprPercent, setAprPercent] = useState("");
  const [nftMint, setNftMint] = useState(""); // borrower only
  const [loading, setLoading] = useState(false);
  const [txSig, setTxSig] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const isLender = side === "lender";
  const label = isLender ? "Lend" : "Borrow";
  const colorClass = isLender ? "border-green-500" : "border-red-500";
  const btnClass = isLender
    ? "bg-green-600 hover:bg-green-500"
    : "bg-red-600 hover:bg-red-500";

  const handleSubmit = async () => {
    if (!publicKey) return;
    setError(null);
    setLoading(true);

    try {
      const principalLamports = BigInt(Math.floor(parseFloat(principalSol) * 1e9));
      const aprBps = BigInt(Math.floor(parseFloat(aprPercent) * 100)); // % → bps

      const tx = new Transaction();

      if (isLender) {
        const ix = buildLenderOrderIx(
          publicKey,
          new PublicKey(orderbook.pubkey),
          principalLamports,
          aprBps,
          0n // GTC
        );
        tx.add(ix);
      } else {
        const ix = buildBorrowerOrderIx(
          publicKey,
          new PublicKey(orderbook.pubkey),
          new PublicKey(nftMint),
          principalLamports,
          aprBps, // this is max_apr_bps
          0n // GTC
        );
        tx.add(ix);
      }

      const sig = await sendTransaction(tx, connection);
      setTxSig(sig);
    } catch (err: any) {
      setError(err.message ?? "Transaction failed");
    } finally {
      setLoading(false);
    }
  };

  if (txSig) {
    return (
      <Modal onClose={onClose} colorClass={colorClass} title={`${label} Order Placed`}>
        <div className="text-center py-4">
          <div className="text-green-400 text-5xl mb-4">✓</div>
          <p className="text-gray-300 mb-2">Order submitted successfully</p>
          <a
            href={`https://solscan.io/tx/${txSig}`}
            target="_blank"
            rel="noreferrer"
            className="text-blue-400 text-sm underline"
          >
            View on Solscan
          </a>
          <br />
          <button
            onClick={onClose}
            className="mt-6 px-6 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg text-white"
          >
            Close
          </button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal onClose={onClose} colorClass={colorClass} title={`Place ${label} Order`}>
      <div className="space-y-4">
        <div className="bg-gray-800 rounded-lg px-4 py-3 text-sm">
          <div className="flex justify-between text-gray-400 mb-1">
            <span>Duration</span>
            <span className="text-white">{formatDuration(orderbook.duration_seconds)}</span>
          </div>
          {orderbook.mid_apr_bps > 0 && (
            <div className="flex justify-between text-gray-400">
              <span>Mid APR (reward center)</span>
              <span className="text-yellow-400">{formatApr(orderbook.mid_apr_bps)}</span>
            </div>
          )}
        </div>

        <div>
          <label className="block text-sm text-gray-400 mb-1">
            {isLender ? "Amount to Lend (SOL)" : "Amount to Borrow (SOL)"}
          </label>
          <input
            type="number"
            value={principalSol}
            onChange={(e) => setPrincipalSol(e.target.value)}
            placeholder="e.g. 1.5"
            className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-500"
          />
        </div>

        <div>
          <label className="block text-sm text-gray-400 mb-1">
            {isLender ? "Offer APR %" : "Max APR you'll pay %"}
          </label>
          <input
            type="number"
            value={aprPercent}
            onChange={(e) => setAprPercent(e.target.value)}
            placeholder={isLender ? "e.g. 20" : "e.g. 35"}
            className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-500"
          />
          {orderbook.mid_apr_bps > 0 && orderbook.max_incentive_spread_bps && (
            <p className="text-xs text-yellow-600 mt-1">
              Stay within {formatApr(orderbook.max_incentive_spread_bps)} of{" "}
              {formatApr(orderbook.mid_apr_bps)} to earn rewards ★
            </p>
          )}
        </div>

        {!isLender && (
          <div>
            <label className="block text-sm text-gray-400 mb-1">NFT Mint Address</label>
            <input
              type="text"
              value={nftMint}
              onChange={(e) => setNftMint(e.target.value)}
              placeholder="Your NFT mint pubkey"
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-white font-mono text-sm focus:outline-none focus:border-blue-500"
            />
            <p className="text-xs text-gray-500 mt-1">
              The NFT will be frozen in your wallet as collateral.
            </p>
          </div>
        )}

        {error && (
          <div className="bg-red-950 border border-red-800 rounded-lg px-3 py-2 text-red-300 text-sm">
            {error}
          </div>
        )}

        <button
          onClick={handleSubmit}
          disabled={loading || !principalSol || !aprPercent || (!isLender && !nftMint)}
          className={`w-full py-3 ${btnClass} disabled:opacity-50 disabled:cursor-not-allowed text-white font-semibold rounded-lg transition-colors`}
        >
          {loading ? "Submitting..." : `Place ${label} Order`}
        </button>
      </div>
    </Modal>
  );
}

function Modal({
  children,
  onClose,
  colorClass,
  title,
}: {
  children: React.ReactNode;
  onClose: () => void;
  colorClass: string;
  title: string;
}) {
  return (
    <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4">
      <div className={`bg-gray-900 border ${colorClass} rounded-2xl w-full max-w-md`}>
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
          <h2 className="text-lg font-semibold text-white">{title}</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white">
            <X size={20} />
          </button>
        </div>
        <div className="px-6 py-5">{children}</div>
      </div>
    </div>
  );
}
