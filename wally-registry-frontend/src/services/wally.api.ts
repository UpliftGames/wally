import { WallyPackageBrief, WallyPackageMetadata } from "../types/wally"

const wallyApiBaseUrl = `${process.env.WALLY_API_URL}/v1`
// API/v1/package-search?query=<query>
const wallyApiSearchUrl = `${wallyApiBaseUrl}/package-search`
// API/v1/package-metadata/<scope>/<name>
const wallyApiMetadataUrl = `${wallyApiBaseUrl}/package-metadata`
// API/v1/package-contents/<scope>/<name>/<version>`
const wallyApiContentsUrl = `${wallyApiBaseUrl}/package-contents`

/**
 * Fetches a list of packages from Wally. The search string is matched against the package scope, name, and description of all available packages
 * A specific field can be matched against by prefixing the searchQuery with the name of that field. Ex: "description: ui"
 * @param {string} searchQuery - The search query as a series of characters
 * @returns {WallyPackageBrief[]} The list of Wally shorthand descriptions that match the searchQuery
 */
export async function getWallyPackages(searchQuery: string | null) {
  if (searchQuery && searchQuery.length > 1) {
    return fetch(
      `${wallyApiSearchUrl}?${new URLSearchParams({
        query: searchQuery,
      })}`
    )
      .then((response) => {
        if (!response.ok) {
          throw new Error("HTTP status " + response.status)
        }
        return response.json()
      })
      .then((data) => data)
      .catch((error) => {})
  } else {
    return []
  }
}

/**
 * Fetches a single package from Wally with all it's corresponding meta information
 * @param {string} packageScope - The owning author or organization of the package
 * @param {string} packageName - The search query as a series of characters
 * @returns {WallyPackageMetadata} The package with it's associated data and dependencies
 */
export async function getWallyPackageMetadata(
  packageScope: string,
  packageName: string
) {
  return fetch(`${wallyApiMetadataUrl}/${packageScope}/${packageName}`)
    .then((response) => {
      if (!response.ok) {
        throw new Error("HTTP status " + response.status)
      }
      return response.json()
    })
    .then((data) => data)
    .catch((error) => {})
}

/**
 * Fetches a single package from Wally with all it's corresponding meta information
 * @param {string} packageScope - The owning author or organization of the package
 * @param {string} packageName - The name of the package
 * @param {string} packageVersion - The semver version string of the desired package
 * @returns {string} The link to the package ZIP
 */
export function buildWallyPackageDownloadLink(
  packageScope: string,
  packageName: string,
  packageVersion: string
) {
  return `${wallyApiContentsUrl}/${packageScope}/${packageName}/${packageVersion}`
}
