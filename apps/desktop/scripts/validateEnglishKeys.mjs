#!/usr/bin/env node

/**
 * Validate English Translation Keys
 *
 * Checks for unused translation keys in English JSON files.
 * Reports keys that are defined but not used in code.
 *
 * Usage:
 *   node scripts/validateEnglishKeys.mjs
 */

import fs from "fs/promises"
import path from "path"
import { fileURLToPath } from "url"
import fg from "fast-glob"
import {
  I18N_PATH,
  SOURCE_LANGUAGE,
  printHeader,
  colors,
  print
} from "./translation-utils.mjs"

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const LOCALE_DIR = path.join(I18N_PATH, SOURCE_LANGUAGE)

/**
 * Get all translation keys from English locale files
 * Extracts keys with _trn_ prefix
 */
async function getAllTranslationKeys() {
  const files = await fs.readdir(LOCALE_DIR)
  const jsonFiles = files.filter(
    (f) =>
      f.endsWith(".json") && !f.includes(".backup") && f !== "languages.json"
  )

  const allKeys = new Set()

  for (const file of jsonFiles) {
    const filePath = path.join(LOCALE_DIR, file)
    const content = await fs.readFile(filePath, "utf-8")
    const translations = JSON.parse(content)

    for (const key of Object.keys(translations)) {
      // Extract just the key part after _trn_ prefix
      if (key.startsWith("_trn_")) {
        const keyPart = key.substring(5) // Remove "_trn_" prefix
        allKeys.add(keyPart)
      } else {
        // Fallback for any keys without prefix (shouldn't happen after migration)
        allKeys.add(key)
      }
    }
  }

  return allKeys
}

/**
 * Extract translation keys from source files
 * Looks for any _trn_ prefixed strings in quotes
 */
async function extractUsedKeys() {
  const usedKeys = new Set()

  // Find all TypeScript/TSX files in entire apps/desktop
  const pattern = `**/*.{ts,tsx}`
  const files = []

  // Scan both directories plus packages/i18n for helpers
  const scanDirs = [
    path.resolve(__dirname, "../../packages/mainWindow/src"),
    path.resolve(__dirname, "../../../packages/ui/src"),
    path.resolve(__dirname, "../../../packages/i18n")
  ]

  for (const dir of scanDirs) {
    try {
      const dirFiles = await fg(pattern, {
        cwd: dir,
        absolute: true,
        ignore: [
          "**/node_modules/**",
          "**/dist/**",
          "**/*.spec.ts",
          "**/*.test.ts",
          "**/*.d.ts"
        ]
      })
      files.push(...dirFiles)
    } catch (_error) {
      // Directory might not exist, skip it
    }
  }

  print(`🔍 Scanning ${files.length} source files...\n`, colors.blue)

  // Pattern to find ANY _trn_ prefixed key
  // Matches strings like "_trn_key", "namespace:_trn_key", etc.
  const trnKeyPattern = /_trn_[a-zA-Z0-9_.-]+/g

  for (const file of files) {
    try {
      const content = await fs.readFile(file, "utf-8")

      let match
      while ((match = trnKeyPattern.exec(content)) !== null) {
        const fullKey = match[0]
        // Extract just the key part after _trn_
        const keyPart = fullKey.substring(5) // Remove "_trn_" prefix
        usedKeys.add(keyPart)
      }
    } catch (_error) {
      // File read error, skip
    }
  }

  return usedKeys
}

/**
 * Main validation function
 */
async function validateEnglishKeys() {
  printHeader("🔍 VALIDATING ENGLISH TRANSLATION KEYS")

  // Load all translation keys
  const allKeys = await getAllTranslationKeys()
  print(`\n📚 Total translation keys in English: ${allKeys.size}`, colors.blue)

  // Extract used keys
  const usedKeys = await extractUsedKeys()
  print(`📝 Translation keys found in code: ${usedKeys.size}\n`, colors.blue)

  // Find unused keys (in translations but not in code)
  const unusedKeys = []
  for (const key of allKeys) {
    if (!usedKeys.has(key)) {
      unusedKeys.push(key)
    }
  }

  // Report results
  print(
    "═══════════════════════════════════════════════════════════",
    colors.cyan
  )

  if (unusedKeys.length === 0) {
    print("✅ No unused translation keys detected!", colors.green)
  } else {
    print(
      `⚠️  Unused keys in English files: ${unusedKeys.length}`,
      colors.yellow
    )
    print("\nKeys defined but not used in code:", colors.yellow)

    const sortedUnused = unusedKeys.sort()
    // Show first 50, then summarize
    for (let i = 0; i < Math.min(50, sortedUnused.length); i++) {
      print(`   - ${sortedUnused[i]}`, colors.yellow)
    }
    if (unusedKeys.length > 50) {
      print(`   ... and ${unusedKeys.length - 50} more`, colors.yellow)
    }
    print(
      "\n⚠️  Note: Some unused keys might be false positives (dynamic keys, etc.)",
      colors.cyan
    )
  }

  print(
    "═══════════════════════════════════════════════════════════\n",
    colors.cyan
  )

  // Exit with error if unused keys found
  if (unusedKeys.length > 0) {
    print(
      "❌ Validation failed: Unused translation keys detected\n",
      colors.bold + colors.red
    )
    process.exit(1)
  }

  print("✅ Validation passed!\n", colors.bold + colors.green)
}

// Run validation
validateEnglishKeys().catch((error) => {
  print(`\n❌ Error: ${error.message}`, colors.bold + colors.red)
  if (error.stack) {
    print(`\n${error.stack}`, colors.red)
  }
  process.exit(1)
})
