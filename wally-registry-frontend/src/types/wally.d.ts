export interface WallyPackageBrief {
  description: string
  name: string
  scope: string
  versions: string[]
}

interface WallyPackageVersionedMetadata {
  package: {
    authors: string[]
    description: string
    license: string
    name: string
    realm: string
    registry: string
    version: string
    homepage: string | null
    repository: string?
  }

  place: {
    "shared-packages"?: string
    "server-packages"?: string
  }

  dependencies: { [index: string]: string }
  "server-dependencies": { [index: string]: string }
  "dev-dependencies": { [index: string]: string }
}

export interface WallyPackageMetadata {
  versions: WallyPackageVersionedMetadata[]
}
