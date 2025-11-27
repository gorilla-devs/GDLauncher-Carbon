#!/usr/bin/env node

/**
 * Sync Translation Files
 *
 * Synchronizes translation files to match English structure:
 * - Adds missing files to languages
 * - Removes extra files from languages
 * - Adds missing keys (with English value as placeholder)
 * - Removes extra keys
 * - Reorders keys to match English order
 *
 * Optional language filter:
 *   node scripts/syncTranslations.mjs --language french
 *   node scripts/syncTranslations.mjs --languages french,german
 */

import fs from "fs"
import path from "path"
import { fileURLToPath } from "url"
import {
  I18N_PATH,
  SOURCE_LANGUAGE,
  getLanguages,
  getTranslationFiles,
  loadJson,
  saveJson,
  getKeys,
  arraysEqual,
  parseLanguageFilter,
  printHeader,
  print,
  colors
} from "./translation-utils.mjs"

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const UNTRANSLATED_PREFIX = "-------- [ UNTRANSLATED ] --------"

/**
 * Validate translations
 */
function validateTranslations(languages) {
  const issues = {
    missingFiles: [],
    extraFiles: [],
    missingKeys: [],
    extraKeys: [],
    wrongOrder: []
  }

  // Get source files
  const sourceFiles = getTranslationFiles(SOURCE_LANGUAGE)

  // Validate each language
  for (const language of languages) {
    if (language === SOURCE_LANGUAGE) continue

    // For new language folders, mark all files as missing
    const languagePath = path.join(I18N_PATH, language)
    const isNewLanguage = !fs.existsSync(languagePath)

    const languageFiles = isNewLanguage ? [] : getTranslationFiles(language)

    // Check for missing files
    for (const sourceFile of sourceFiles) {
      if (!languageFiles.includes(sourceFile)) {
        issues.missingFiles.push({ language, file: sourceFile })
      }
    }

    // Check for extra files
    for (const languageFile of languageFiles) {
      if (!sourceFiles.includes(languageFile)) {
        issues.extraFiles.push({ language, file: languageFile })
      }
    }

    // Validate each file
    for (const filename of sourceFiles) {
      if (!languageFiles.includes(filename)) continue

      const sourceData = loadJson(SOURCE_LANGUAGE, filename)
      const targetData = loadJson(language, filename)

      const sourceKeys = getKeys(sourceData)
      const targetKeys = getKeys(targetData)

      // Check for missing keys
      const missingKeys = sourceKeys.filter((key) => !(key in targetData))
      if (missingKeys.length > 0) {
        issues.missingKeys.push({ language, file: filename, keys: missingKeys })
      }

      // Check for extra keys
      const extraKeys = targetKeys.filter((key) => !(key in sourceData))
      if (extraKeys.length > 0) {
        issues.extraKeys.push({ language, file: filename, keys: extraKeys })
      }

      // Check key order (only if no missing/extra keys)
      if (missingKeys.length === 0 && extraKeys.length === 0) {
        if (!arraysEqual(sourceKeys, targetKeys)) {
          issues.wrongOrder.push({ language, file: filename })
        }
      }
    }
  }

  return issues
}

/**
 * Add UNTRANSLATED prefix to all values in an object
 */
function prefixWithUntranslated(obj) {
  const prefixed = {}
  for (const [key, value] of Object.entries(obj)) {
    if (typeof value === "string") {
      prefixed[key] = `${UNTRANSLATED_PREFIX} ${value}`
    } else {
      prefixed[key] = value
    }
  }
  return prefixed
}

/**
 * Fix translation issues
 */
function fixTranslations(issues, languages) {
  printHeader("🔧 FIXING TRANSLATIONS")

  let fixCount = 0

  // Create language folders if they don't exist
  for (const language of languages) {
    if (language === SOURCE_LANGUAGE) continue
    const languagePath = path.join(I18N_PATH, language)
    if (!fs.existsSync(languagePath)) {
      fs.mkdirSync(languagePath, { recursive: true })
      print(`\n📁 Created new language folder: ${language}`, colors.green)
    }
  }

  // Fix missing files
  for (const { language, file } of issues.missingFiles) {
    print(`\n📄 Creating missing file: ${language}/${file}`, colors.yellow)
    const sourceData = loadJson(SOURCE_LANGUAGE, file)
    // Prefix all values with UNTRANSLATED marker for new files
    const prefixedData = prefixWithUntranslated(sourceData)
    saveJson(language, file, prefixedData)
    fixCount++
  }

  // Fix extra files
  for (const { language, file } of issues.extraFiles) {
    print(`\n🗑️  Removing extra file: ${language}/${file}`, colors.yellow)
    const filePath = path.join(I18N_PATH, language, file)
    fs.unlinkSync(filePath)
    fixCount++
  }

  // Get all languages and files for key fixes
  const sourceFiles = getTranslationFiles(SOURCE_LANGUAGE)

  // Fix missing keys, extra keys, and wrong order
  for (const language of languages) {
    if (language === SOURCE_LANGUAGE) continue

    for (const filename of sourceFiles) {
      const filePath = path.join(I18N_PATH, language, filename)
      if (!fs.existsSync(filePath)) continue // File will be created above

      const sourceData = loadJson(SOURCE_LANGUAGE, filename)
      const targetData = loadJson(language, filename)

      // Find issues for this specific file
      const missingKeysIssue = issues.missingKeys.find(
        (i) => i.language === language && i.file === filename
      )
      const extraKeysIssue = issues.extraKeys.find(
        (i) => i.language === language && i.file === filename
      )
      const wrongOrderIssue = issues.wrongOrder.find(
        (i) => i.language === language && i.file === filename
      )

      if (missingKeysIssue || extraKeysIssue || wrongOrderIssue) {
        print(`\n🔧 Fixing: ${language}/${filename}`, colors.yellow)

        // Create new object with correct keys in correct order
        const fixedData = {}
        const sourceKeys = getKeys(sourceData)

        for (const key of sourceKeys) {
          if (key in targetData) {
            // Keep existing translation
            fixedData[key] = targetData[key]
          } else {
            // Add missing key with English value, prefixed with UNTRANSLATED marker
            const englishValue = sourceData[key]
            fixedData[key] =
              typeof englishValue === "string"
                ? `${UNTRANSLATED_PREFIX} ${englishValue}`
                : englishValue
            print(`   + Added key: ${key}`, colors.green)
            fixCount++
          }
        }

        // Report removed keys
        const targetKeys = getKeys(targetData)
        for (const key of targetKeys) {
          if (!(key in sourceData)) {
            print(`   - Removed key: ${key}`, colors.red)
            fixCount++
          }
        }

        // Report reordering if applicable
        if (wrongOrderIssue && !missingKeysIssue && !extraKeysIssue) {
          print(`   ↕ Reordered keys to match English`, colors.cyan)
          fixCount++
        }

        // Save fixed file
        saveJson(language, filename, fixedData)
      }
    }
  }

  return fixCount
}

/**
 * Print summary
 */
function printSummary(issues) {
  printHeader("📊 SUMMARY")

  const totalIssues =
    issues.missingFiles.length +
    issues.extraFiles.length +
    issues.missingKeys.length +
    issues.extraKeys.length +
    issues.wrongOrder.length

  if (totalIssues === 0) {
    print("\n✅ All translations are valid!", colors.bold + colors.green)
    return true
  }

  print(
    `\n⚠️  Found ${totalIssues} issues to fix:`,
    colors.bold + colors.yellow
  )

  if (issues.missingFiles.length > 0) {
    print(`   - ${issues.missingFiles.length} missing files`, colors.yellow)
  }
  if (issues.extraFiles.length > 0) {
    print(`   - ${issues.extraFiles.length} extra files`, colors.yellow)
  }
  if (issues.missingKeys.length > 0) {
    const totalMissing = issues.missingKeys.reduce(
      (sum, i) => sum + i.keys.length,
      0
    )
    print(
      `   - ${totalMissing} missing keys across ${issues.missingKeys.length} files`,
      colors.yellow
    )
  }
  if (issues.extraKeys.length > 0) {
    const totalExtra = issues.extraKeys.reduce(
      (sum, i) => sum + i.keys.length,
      0
    )
    print(
      `   - ${totalExtra} extra keys across ${issues.extraKeys.length} files`,
      colors.yellow
    )
  }
  if (issues.wrongOrder.length > 0) {
    print(
      `   - ${issues.wrongOrder.length} files with wrong key order`,
      colors.yellow
    )
  }

  return false
}

/**
 * Main function
 */
async function main() {
  try {
    print("\n🚀 Translation Sync Script", colors.bold + colors.cyan)
    print(`I18n Path: ${I18N_PATH}`, colors.blue)

    // Parse language filter
    const languageFilter = parseLanguageFilter()

    // Get existing languages
    let languages = getLanguages()

    // If language filter is provided, allow creating new languages
    if (languageFilter && languageFilter.length > 0) {
      const existingLanguages = languages.filter(
        (lang) => lang === SOURCE_LANGUAGE || languageFilter.includes(lang)
      )
      // Add requested languages that don't exist yet
      for (const lang of languageFilter) {
        if (!existingLanguages.includes(lang)) {
          existingLanguages.push(lang)
        }
      }
      languages = existingLanguages
    }

    // Validate
    const issues = validateTranslations(languages)
    const isValid = printSummary(issues)

    if (isValid) {
      print("\n✨ Nothing to fix!\n", colors.green)
      process.exit(0)
    }

    // Fix issues
    const fixCount = fixTranslations(issues, languages)

    printHeader("✅ COMPLETED")
    print(
      `\n✨ Fixed ${fixCount} issues successfully!\n`,
      colors.bold + colors.green
    )
    process.exit(0)
  } catch (error) {
    print(`\n❌ Error: ${error.message}`, colors.bold + colors.red)
    if (error.stack) {
      print(`\n${error.stack}`, colors.red)
    }
    process.exit(1)
  }
}

// Run
main()
