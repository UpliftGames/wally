/**
 * Capitalizes the first letter of a string
 * @param {string} text - The string to be capitalized
 * @returns {string} The capitalized string
 */
export default function capitalize(text: string | undefined) {
  return text ? text[0].toUpperCase() + text.substring(1) : ""
}
