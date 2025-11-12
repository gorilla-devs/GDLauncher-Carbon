#!/usr/bin/env node

/**
 * Shared Translation Utilities
 *
 * Common functions used by all translation validation and sync scripts.
 */

import fs from "fs"
import path from "path"
import { fileURLToPath } from "url"

// Get __dirname equivalent in ESM
const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Configuration
export const REPO_ROOT = path.resolve(__dirname, "../../../")
export const I18N_PATH = path.join(REPO_ROOT, "packages/i18n/locale")
export const SOURCE_LANGUAGE = "english"

// ANSI color codes
export const colors = {
  reset: "\x1b[0m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
  bold: "\x1b[1m"
}

/**
 * Print colored message
 */
export function print(message, color = "") {
  console.log(`${color}${message}${colors.reset}`)
}

/**
 * Print section header
 */
export function printHeader(message) {
  print(`\n${"=".repeat(80)}`, colors.bold)
  print(message, colors.bold + colors.cyan)
  print("=".repeat(80), colors.bold)
}

/**
 * Get all language directories
 */
export function getLanguages(filter = null) {
  const entries = fs.readdirSync(I18N_PATH, { withFileTypes: true })
  let languages = entries
    .filter((entry) => entry.isDirectory() && !entry.name.startsWith("."))
    .map((entry) => entry.name)

  if (!languages.includes(SOURCE_LANGUAGE)) {
    throw new Error(
      `Source language "${SOURCE_LANGUAGE}" not found in ${I18N_PATH}`
    )
  }

  // Filter to specific languages if provided
  if (filter && filter.length > 0) {
    languages = languages.filter((lang) => filter.includes(lang))
  }

  return languages
}

/**
 * Get all JSON files in a language directory
 */
export function getTranslationFiles(language) {
  const languagePath = path.join(I18N_PATH, language)
  const entries = fs.readdirSync(languagePath)
  return entries.filter((file) => file.endsWith(".json")).sort()
}

/**
 * Load and parse a JSON file
 */
export function loadJson(language, filename) {
  const filePath = path.join(I18N_PATH, language, filename)
  try {
    const content = fs.readFileSync(filePath, "utf-8")
    return JSON.parse(content)
  } catch (error) {
    throw new Error(`Failed to parse ${language}/${filename}: ${error.message}`)
  }
}

/**
 * Save JSON file with consistent formatting
 */
export function saveJson(language, filename, data) {
  const filePath = path.join(I18N_PATH, language, filename)
  const content = JSON.stringify(data, null, 2) + "\n"
  fs.writeFileSync(filePath, content, "utf-8")
}

/**
 * Get keys from an object in order
 */
export function getKeys(obj) {
  return Object.keys(obj)
}

/**
 * Check if two arrays have the same elements in the same order
 */
export function arraysEqual(arr1, arr2) {
  if (arr1.length !== arr2.length) return false
  return arr1.every((value, index) => value === arr2[index])
}

/**
 * Parse language filter from command line
 * Supports: --language fr or --languages fr,de
 */
export function parseLanguageFilter() {
  const languageIndex = process.argv.indexOf("--language")
  const languagesIndex = process.argv.indexOf("--languages")

  if (languageIndex !== -1) {
    const lang = process.argv[languageIndex + 1]
    return lang ? [lang] : null
  }

  if (languagesIndex !== -1) {
    const langs = process.argv[languagesIndex + 1]
    return langs ? langs.split(",").map((l) => l.trim()) : null
  }

  return null
}

/**
 * Validate that a language exists
 */
export function validateLanguageExists(language) {
  const languages = getLanguages()
  if (!languages.includes(language)) {
    throw new Error(
      `Language "${language}" not found. Available: ${languages.join(", ")}`
    )
  }
}
