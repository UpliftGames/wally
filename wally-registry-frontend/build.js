// Entrypoint for building the static site.
//
// This script bundles the static site generator using Parcel, which in turn
// will invoke Parcel again itself, reusing the same assets.

const { Parcel } = require("@parcel/core")

process.env.NODE_ENV = "production"

const publicUrl = process.env.PARCEL_PUBLIC_URL || "/"

console.log("Bundling client with Parcel...")

const bundler = new Parcel({
  entries: ["src/index.html"],
  defaultConfig: "@parcel/config-default",
  defaultTargetOptions: {
    distDir: "build/static",
    publicUrl: "/static",
  },
  logLevel: 2,
})

bundler.run().then(() => {})
