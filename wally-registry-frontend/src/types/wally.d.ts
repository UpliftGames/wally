export interface WallyPackageBrief {
  description: string
  name: string
  scope: string
  versions: string[]
}

export interface WallyPackageMetadata {
  dependencies: {}
  "dev-dependencies": {}
  package: {
    authors: string[]
    description: string
    license: string
    name: string
    realm: string
    registry: string
    version: string
  }
  "server-dependences": {}
}
