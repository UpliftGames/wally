/**
 * Capitalizes the first letter of a string.
 * @param {string} text - The string to be capitalized.
 */
export default function capitalize(text: string) {
  return text[0].toUpperCase() + text.substring(1)
}
