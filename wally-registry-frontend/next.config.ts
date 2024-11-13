import createMDX from "@next/mdx"
import type { NextConfig } from "next"
import { setupDevPlatform } from '@cloudflare/next-on-pages/next-dev';

const nextConfig: NextConfig = {
  pageExtensions: ["js", "jsx", "md", "mdx", "ts", "tsx"],
  compiler: {
    styledComponents: true,
  },
}

const withMDX = createMDX({
  // Add markdown plugins here, as desired
})

if (process.env.NODE_ENV === 'development') {
  await setupDevPlatform();
}

export default withMDX(nextConfig)
