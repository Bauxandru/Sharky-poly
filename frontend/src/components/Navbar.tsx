"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import clsx from "clsx";

export function Navbar() {
  const pathname = usePathname();

  const links = [
    { href: "/", label: "Orderbooks" },
    { href: "/rewards", label: "Rewards" },
    { href: "/portfolio", label: "Portfolio" },
  ];

  return (
    <nav className="border-b border-gray-800 bg-gray-950/80 backdrop-blur sticky top-0 z-40">
      <div className="max-w-7xl mx-auto px-4 flex items-center justify-between h-16">
        <div className="flex items-center gap-8">
          <Link href="/" className="text-lg font-bold text-white">
            NFT Lending
          </Link>
          <div className="hidden md:flex gap-1">
            {links.map((l) => (
              <Link
                key={l.href}
                href={l.href}
                className={clsx(
                  "px-3 py-2 rounded-lg text-sm transition-colors",
                  pathname === l.href
                    ? "bg-gray-800 text-white"
                    : "text-gray-400 hover:text-white hover:bg-gray-800/50"
                )}
              >
                {l.label}
              </Link>
            ))}
          </div>
        </div>
        <WalletMultiButton className="!bg-blue-600 hover:!bg-blue-500 !rounded-lg !text-sm !py-2 !px-4" />
      </div>
    </nav>
  );
}
