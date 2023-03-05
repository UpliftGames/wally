export interface WallyPackageBrief {
  description: string
  name: string
  scope: string
  versions: string[]
}

export interface WallyPackageMetadata {
  dependencies: { string: string }
  "dev-dependencies": { string: string }
  package: {
    authors: string[]
    description: string
    license: string
    name: string
    realm: string
    registry: string
    version: string
  }
  "server-dependencies": { string: string }
}
