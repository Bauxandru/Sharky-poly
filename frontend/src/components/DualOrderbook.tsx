"use client";

import type { LenderOrder, BorrowerOrder } from "@/lib/types";
import { formatApr, formatSol } from "@/lib/format";
import clsx from "clsx";

interface Props {
  lenderOrders: LenderOrder[];
  borrowerOrders: BorrowerOrder[];
  midAprBps: number;
  maxSpreadBps: number;
}

/**
 * Dual Orderbook Component
 *
 * Displays the lender (green) and borrower (red) sides side-by-side,
 * similar to a traditional CLOB. Mid-APR is shown in the center.
 *
 * Lender orders sorted ascending by APR (best=lowest first, at top).
 * Borrower orders sorted descending by max APR (best=highest first, at top).
 */
export function DualOrderbook({
  lenderOrders,
  borrowerOrders,
  midAprBps,
  maxSpreadBps,
}: Props) {
  const DISPLAY_ROWS = 10;

  // Best lender orders (lowest APR first) — shown closest to center
  const topLenders = [...lenderOrders]
    .sort((a, b) => a.apr_bps - b.apr_bps)
    .slice(0, DISPLAY_ROWS)
    .reverse(); // reverse so best (lowest APR) is at bottom, closest to mid

  // Best borrower orders (highest max APR first) — closest to center
  const topBorrowers = [...borrowerOrders]
    .sort((a, b) => b.max_apr_bps - a.max_apr_bps)
    .slice(0, DISPLAY_ROWS);

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl overflow-hidden">
      <div className="grid grid-cols-2 divide-x divide-gray-800">
        {/* Lender Side (Left) */}
        <div>
          <div className="px-4 py-2 bg-green-950/40 border-b border-gray-800">
            <h3 className="text-green-400 font-semibold text-sm">
              Lender Offers ({lenderOrders.length})
            </h3>
            <p className="text-gray-500 text-xs">Sorted: lowest APR first (best for borrowers)</p>
          </div>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-gray-500 text-xs border-b border-gray-800">
                <th className="px-4 py-2 text-left">APR</th>
                <th className="px-4 py-2 text-right">Amount (SOL)</th>
                <th className="px-4 py-2 text-right">Reward</th>
              </tr>
            </thead>
            <tbody>
              {topLenders.map((order) => {
                const spread = Math.abs(order.apr_bps - midAprBps);
                const inRange = maxSpreadBps > 0 && spread < maxSpreadBps;
                return (
                  <tr
                    key={order.pubkey}
                    className={clsx(
                      "border-b border-gray-800/50",
                      inRange ? "bg-green-950/30" : ""
                    )}
                  >
                    <td className="px-4 py-2 text-green-400 font-mono font-medium">
                      {formatApr(order.apr_bps)}
                      {inRange && (
                        <span className="ml-1 text-yellow-500 text-xs" title="Earning rewards">★</span>
                      )}
                    </td>
                    <td className="px-4 py-2 text-right text-gray-300 font-mono">
                      {formatSol(order.principal_lamports)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {inRange ? (
                        <span className="text-yellow-400 text-xs">Active</span>
                      ) : (
                        <span className="text-gray-600 text-xs">—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
              {topLenders.length === 0 && (
                <tr>
                  <td colSpan={3} className="px-4 py-6 text-center text-gray-600 text-xs">
                    No lender offers
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Borrower Side (Right) */}
        <div>
          <div className="px-4 py-2 bg-red-950/40 border-b border-gray-800">
            <h3 className="text-red-400 font-semibold text-sm">
              Borrower Asks ({borrowerOrders.length})
            </h3>
            <p className="text-gray-500 text-xs">Sorted: highest max-APR first (best for lenders)</p>
          </div>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-gray-500 text-xs border-b border-gray-800">
                <th className="px-4 py-2 text-left">Max APR</th>
                <th className="px-4 py-2 text-right">Amount (SOL)</th>
                <th className="px-4 py-2 text-right">Reward</th>
              </tr>
            </thead>
            <tbody>
              {topBorrowers.map((order) => {
                const spread = Math.abs(order.max_apr_bps - midAprBps);
                const inRange = maxSpreadBps > 0 && spread < maxSpreadBps;
                return (
                  <tr
                    key={order.pubkey}
                    className={clsx(
                      "border-b border-gray-800/50",
                      inRange ? "bg-red-950/30" : ""
                    )}
                  >
                    <td className="px-4 py-2 text-red-400 font-mono font-medium">
                      {formatApr(order.max_apr_bps)}
                      {inRange && (
                        <span className="ml-1 text-yellow-500 text-xs" title="Earning rewards">★</span>
                      )}
                    </td>
                    <td className="px-4 py-2 text-right text-gray-300 font-mono">
                      {formatSol(order.principal_lamports)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {inRange ? (
                        <span className="text-yellow-400 text-xs">Active</span>
                      ) : (
                        <span className="text-gray-600 text-xs">—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
              {topBorrowers.length === 0 && (
                <tr>
                  <td colSpan={3} className="px-4 py-6 text-center text-gray-600 text-xs">
                    No borrower asks
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Mid-APR Separator */}
      {midAprBps > 0 && (
        <div className="px-4 py-2 bg-yellow-950/30 border-t border-yellow-800/40 text-center">
          <span className="text-yellow-400 font-bold text-sm">
            Mid APR: {formatApr(midAprBps)}
          </span>
          {maxSpreadBps > 0 && (
            <span className="text-gray-500 text-xs ml-3">
              Reward range: {formatApr(Math.max(0, midAprBps - maxSpreadBps))} –{" "}
              {formatApr(midAprBps + maxSpreadBps)}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
