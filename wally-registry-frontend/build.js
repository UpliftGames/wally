// Entrypoint for building the static site.
//
// This script bundles the static site generator using Parcel, which in turn
// will invoke Parcel again itself, reusing the same assets.

const Parcel = require("parcel-bundler")

process.env.NODE_ENV = "production"

const publicUrl = process.env.PARCEL_PUBLIC_URL || "/"

console.log("Bundling client with Parcel...")

const bundler = new Parcel(["src/index.html"], {
  outDir: "build/static",
  publicUrl: "/static",
  watch: false,
  autoInstall: false,
  logLevel: 2,
})

bundler.bundle().then(() => {})
