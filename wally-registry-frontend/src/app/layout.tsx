import Footer from "@/components/Footer"
import Header from "@/components/Header"
import StyledComponentsRegistry from "@/lib/registry"
import type { Metadata, Viewport } from "next"
import localFont from "next/font/local"
import "./fonts/icons.css"
import "./globals.css"
import "./theme.css"

export const metadata: Metadata = {
  title: { template: "%s | Wally", default: "Wally" },
  description:
    "Wally is a modern package manager for Roblox projects inspired by Cargo",
}

export const viewport: Viewport = {
  themeColor: "#FF6A8B",
}

const iosevka = localFont({
  src: [
    {
      path: "./fonts/iosevka/iosevka-extended.woff2",
      weight: "400",
      style: "normal",
    },
    {
      path: "./fonts/iosevka/iosevka-extendedbold.woff2",
      weight: "700",
      style: "normal",
    },
    {
      path: "./fonts/iosevka/iosevka-extendedboldoblique.woff2",
      weight: "700",
      style: "oblique",
    },
    {
      path: "./fonts/iosevka/iosevka-extendedheavy.woff2",
      weight: "900",
      style: "normal",
    },
    {
      path: "./fonts/iosevka/iosevka-extendedheavyoblique.woff2",
      weight: "900",
      style: "oblique",
    },
    {
      path: "./fonts/iosevka/iosevka-extendedlight.woff2",
      weight: "300",
      style: "normal",
    },
    {
      path: "./fonts/iosevka/iosevka-extendedsemibolditalic.woff2",
      weight: "600",
      style: "italic",
    },
  ],
  display: "swap",
  fallback: ["Hack", "Consolas", "monospace"],
})

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" className={`${iosevka.className}`}>
      <body>
        {/* Next.js doesn't need a wrapper #app tag, but for our legacy CSS we will keep the convention around   */}
        <div id="app">
          <StyledComponentsRegistry>
            <Header />

            {children}

            <Footer />
          </StyledComponentsRegistry>
        </div>
      </body>
    </html>
  )
}
