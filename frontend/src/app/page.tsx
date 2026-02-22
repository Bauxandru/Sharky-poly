"use client";

import useSWR from "swr";
import Link from "next/link";
import { fetcher, API_URL } from "@/lib/api";
import { formatApr, formatSol, formatDuration } from "@/lib/format";
import type { OrderBook } from "@/lib/types";

export default function Home() {
  const { data: orderbooks, isLoading } = useSWR<OrderBook[]>(
    `${API_URL}/api/orderbooks`,
    fetcher,
    { refreshInterval: 10_000 }
  );

  return (
    <div>
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-white mb-2">NFT Lending Orderbooks</h1>
        <p className="text-gray-400">
          Dual-sided CLOB — both lenders and borrowers post limit orders.
          Keep orders near the mid-APR to earn liquidity rewards.
        </p>
      </div>

      {isLoading ? (
        <OrderbookSkeleton />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-gray-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-gray-400 border-b border-gray-800 bg-gray-900">
                <th className="px-4 py-3">Collection</th>
                <th className="px-4 py-3">Duration</th>
                <th className="px-4 py-3 text-green-400">Best Lender APR</th>
                <th className="px-4 py-3 text-red-400">Best Borrower APR</th>
                <th className="px-4 py-3 text-yellow-400">Mid APR</th>
                <th className="px-4 py-3">Lender Orders</th>
                <th className="px-4 py-3">Borrower Orders</th>
                <th className="px-4 py-3">Active Loans</th>
                <th className="px-4 py-3">Volume</th>
              </tr>
            </thead>
            <tbody>
              {(orderbooks ?? []).map((ob) => (
                <tr
                  key={ob.pubkey}
                  className="border-b border-gray-800 hover:bg-gray-900 transition-colors"
                >
                  <td className="px-4 py-3">
                    <Link
                      href={`/orderbook/${ob.pubkey}`}
                      className="font-medium text-white hover:text-blue-400 transition-colors"
                    >
                      {ob.collection_mint.slice(0, 8)}...
                    </Link>
                  </td>
                  <td className="px-4 py-3 text-gray-300">
                    {formatDuration(ob.duration_seconds)}
                  </td>
                  <td className="px-4 py-3 apr-lender">
                    {ob.best_lender_apr_bps > 0
                      ? formatApr(ob.best_lender_apr_bps)
                      : <span className="text-gray-600">—</span>}
                  </td>
                  <td className="px-4 py-3 apr-borrower">
                    {ob.best_borrower_max_apr_bps > 0
                      ? formatApr(ob.best_borrower_max_apr_bps)
                      : <span className="text-gray-600">—</span>}
                  </td>
                  <td className="px-4 py-3 apr-mid">
                    {ob.mid_apr_bps > 0 ? formatApr(ob.mid_apr_bps) : "—"}
                  </td>
                  <td className="px-4 py-3 text-green-400">
                    {ob.open_lender_orders}
                  </td>
                  <td className="px-4 py-3 text-red-400">
                    {ob.open_borrower_orders}
                  </td>
                  <td className="px-4 py-3 text-gray-300">{ob.active_loans}</td>
                  <td className="px-4 py-3 text-gray-300">
                    {formatSol(ob.total_volume_lamports)} SOL
                  </td>
                </tr>
              ))}
              {orderbooks?.length === 0 && (
                <tr>
                  <td colSpan={9} className="px-4 py-8 text-center text-gray-500">
                    No orderbooks yet. Be the first to create one.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function OrderbookSkeleton() {
  return (
    <div className="space-y-2">
      {[1, 2, 3, 4, 5].map((i) => (
        <div key={i} className="h-12 bg-gray-900 rounded animate-pulse" />
      ))}
    </div>
  );
}
