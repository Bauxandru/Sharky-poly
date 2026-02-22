"use client";

import { useState } from "react";
import useSWR from "swr";
import { useParams } from "next/navigation";
import { fetcher, API_URL } from "@/lib/api";
import { formatApr, formatSol, formatDuration, formatTs } from "@/lib/format";
import type { OrderBook, LenderOrder, BorrowerOrder, Loan } from "@/lib/types";
import { DualOrderbook } from "@/components/DualOrderbook";
import { PlaceOrderModal } from "@/components/PlaceOrderModal";
import { RewardBadge } from "@/components/RewardBadge";
import { useWallet } from "@solana/wallet-adapter-react";

export default function OrderbookPage() {
  const { pubkey } = useParams<{ pubkey: string }>();
  const { connected } = useWallet();
  const [showModal, setShowModal] = useState<"lender" | "borrower" | null>(null);

  const { data: ob } = useSWR<OrderBook>(
    `${API_URL}/api/orderbooks/${pubkey}`,
    fetcher,
    { refreshInterval: 5_000 }
  );

  const { data: lenderOrders } = useSWR<LenderOrder[]>(
    `${API_URL}/api/orderbooks/${pubkey}/lender-orders`,
    fetcher,
    { refreshInterval: 5_000 }
  );

  const { data: borrowerOrders } = useSWR<BorrowerOrder[]>(
    `${API_URL}/api/orderbooks/${pubkey}/borrower-orders`,
    fetcher,
    { refreshInterval: 5_000 }
  );

  const { data: loans } = useSWR<Loan[]>(
    `${API_URL}/api/orderbooks/${pubkey}/loans`,
    fetcher,
    { refreshInterval: 15_000 }
  );

  if (!ob) {
    return <div className="animate-pulse h-64 bg-gray-900 rounded-xl" />;
  }

  const spread = ob.best_borrower_max_apr_bps - ob.best_lender_apr_bps;
  const isMatchable =
    ob.best_lender_apr_bps > 0 &&
    ob.best_borrower_max_apr_bps > 0 &&
    spread >= 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">
            {ob.collection_mint.slice(0, 12)}...
          </h1>
          <p className="text-gray-400 mt-1">
            {formatDuration(ob.duration_seconds)} loans
          </p>
        </div>

        <div className="flex gap-3">
          {connected ? (
            <>
              <button
                onClick={() => setShowModal("lender")}
                className="px-5 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg font-medium transition-colors"
              >
                Lend (Place Offer)
              </button>
              <button
                onClick={() => setShowModal("borrower")}
                className="px-5 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg font-medium transition-colors"
              >
                Borrow (Place Ask)
              </button>
            </>
          ) : (
            <p className="text-gray-500 text-sm pt-2">Connect wallet to trade</p>
          )}
        </div>
      </div>

      {/* Stats bar */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        <StatCard label="Best Lender APR" value={ob.best_lender_apr_bps > 0 ? formatApr(ob.best_lender_apr_bps) : "—"} color="green" />
        <StatCard label="Mid APR" value={ob.mid_apr_bps > 0 ? formatApr(ob.mid_apr_bps) : "—"} color="yellow" />
        <StatCard label="Best Borrower APR" value={ob.best_borrower_max_apr_bps > 0 ? formatApr(ob.best_borrower_max_apr_bps) : "—"} color="red" />
        <StatCard label="Active Loans" value={ob.active_loans.toString()} color="blue" />
        <StatCard label="Total Volume" value={`${formatSol(ob.total_volume_lamports)} SOL`} color="gray" />
      </div>

      {/* Reward eligibility notice */}
      {ob.mid_apr_bps > 0 && ob.max_incentive_spread_bps && (
        <div className="bg-yellow-950/30 border border-yellow-800 rounded-lg px-4 py-3 text-sm">
          <span className="text-yellow-400 font-medium">Liquidity Rewards Active.</span>
          <span className="text-gray-300 ml-2">
            Place orders within ±{formatApr(ob.max_incentive_spread_bps)} of the mid-APR (
            {formatApr(ob.mid_apr_bps)}) to earn rewards.
            Score is quadratic — closer to mid earns exponentially more.
          </span>
        </div>
      )}

      {/* Dual Orderbook */}
      <DualOrderbook
        lenderOrders={lenderOrders ?? []}
        borrowerOrders={borrowerOrders ?? []}
        midAprBps={ob.mid_apr_bps}
        maxSpreadBps={ob.max_incentive_spread_bps ?? 0}
      />

      {/* Recent Loans */}
      <div>
        <h2 className="text-lg font-semibold text-white mb-3">Recent Loans</h2>
        <div className="overflow-x-auto rounded-xl border border-gray-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-gray-400 border-b border-gray-800 bg-gray-900">
                <th className="px-4 py-2">NFT</th>
                <th className="px-4 py-2">Principal</th>
                <th className="px-4 py-2">APR</th>
                <th className="px-4 py-2">Due</th>
                <th className="px-4 py-2">State</th>
              </tr>
            </thead>
            <tbody>
              {(loans ?? []).map((loan) => (
                <tr key={loan.pubkey} className="border-b border-gray-800 hover:bg-gray-900">
                  <td className="px-4 py-2 font-mono text-xs text-gray-300">
                    {loan.nft_mint.slice(0, 8)}...
                  </td>
                  <td className="px-4 py-2">{formatSol(loan.principal_lamports)} SOL</td>
                  <td className="px-4 py-2 text-green-400">{formatApr(loan.apr_bps)}</td>
                  <td className="px-4 py-2 text-gray-400">{formatTs(loan.end_ts)}</td>
                  <td className="px-4 py-2">
                    <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                      loan.state === "active" ? "bg-blue-900 text-blue-300" :
                      loan.state === "repaid" ? "bg-green-900 text-green-300" :
                      "bg-red-900 text-red-300"
                    }`}>
                      {loan.state}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Order Modal */}
      {showModal && (
        <PlaceOrderModal
          orderbook={ob}
          side={showModal}
          onClose={() => setShowModal(null)}
        />
      )}
    </div>
  );
}

function StatCard({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: "green" | "red" | "yellow" | "blue" | "gray";
}) {
  const colorMap = {
    green: "text-green-400",
    red: "text-red-400",
    yellow: "text-yellow-400",
    blue: "text-blue-400",
    gray: "text-gray-300",
  };

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-lg px-4 py-3">
      <p className="text-gray-500 text-xs mb-1">{label}</p>
      <p className={`text-lg font-bold ${colorMap[color]}`}>{value}</p>
    </div>
  );
}
