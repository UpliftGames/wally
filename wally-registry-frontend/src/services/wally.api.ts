const wallyApiBaseUrl = "https://api.test.wally.run/v1"
// https://api.test.wally.run/v1/package-search/<query>
const wallyApiSearchUrl = `${wallyApiBaseUrl}/package-search`
// https://api.test.wally.run/v1/package-metadata/<scope>/<name>
const wallyApiMetadataUrl = `${wallyApiBaseUrl}/package-metadata`

export async function getWallyPackages(searchQuery: string | null) {
  if (searchQuery && searchQuery.length > 1) {
    return fetch(`${wallyApiSearchUrl}/${searchQuery}`)
      .then((response) => response.json())
      .then((data) => data)
  } else {
    return []
  }
}

export async function getWallyPackageMetadata(
  packageScope: string,
  packageName: string
) {
  return fetch(`${wallyApiMetadataUrl}/${packageScope}/${packageName}`)
    .then((response) => response.json())
    .then((data) => data.versions[0])
}
