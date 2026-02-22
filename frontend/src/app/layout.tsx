import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { WalletProviders } from "@/components/WalletProviders";
import { Navbar } from "@/components/Navbar";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "NFT Lending | Dual Orderbook",
  description: "P2P NFT-backed lending with dual orderbook and liquidity rewards",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <WalletProviders>
          <div className="min-h-screen bg-gray-950 text-gray-100">
            <Navbar />
            <main className="max-w-7xl mx-auto px-4 py-8">{children}</main>
          </div>
        </WalletProviders>
      </body>
    </html>
  );
}
