#!/usr/bin/env node

/**
 * Validate Translation Synchronization
 *
 * Validates that all languages are synchronized with English:
 * - All languages have the same files as English
 * - All files have the same keys as English
 * - Keys are in the same order as English
 * - No extra keys in non-English languages
 *
 * Optional language filter:
 *   node scripts/validateTranslationSync.mjs --language french
 *   node scripts/validateTranslationSync.mjs --languages french,german
 */

import {
  I18N_PATH,
  SOURCE_LANGUAGE,
  getLanguages,
  getTranslationFiles,
  loadJson,
  getKeys,
  arraysEqual,
  parseLanguageFilter,
  validateLanguageExists,
  printHeader,
  print,
  colors
} from "./translation-utils.mjs"

/**
 * Validate translations for specified languages
 */
function validateTranslations(languages) {
  printHeader("🔍 VALIDATING TRANSLATION SYNC")

  print(
    `\nValidating ${languages.length} language(s): ${languages.join(", ")}`,
    colors.blue
  )
  print(`Source language: ${SOURCE_LANGUAGE}`, colors.blue)

  const issues = {
    missingFiles: [],
    extraFiles: [],
    missingKeys: [],
    extraKeys: [],
    wrongOrder: []
  }

  // Get source files
  const sourceFiles = getTranslationFiles(SOURCE_LANGUAGE)
  print(`\nSource files (${sourceFiles.length}):`, colors.blue)
  sourceFiles.forEach((file) => print(`  - ${file}`, colors.blue))

  // Validate each language
  for (const language of languages) {
    if (language === SOURCE_LANGUAGE) continue

    print(`\n${"─".repeat(80)}`, colors.cyan)
    print(`Validating: ${language}`, colors.bold + colors.cyan)
    print("─".repeat(80), colors.cyan)

    const languageFiles = getTranslationFiles(language)

    // Check for missing files
    for (const sourceFile of sourceFiles) {
      if (!languageFiles.includes(sourceFile)) {
        issues.missingFiles.push({ language, file: sourceFile })
        print(`  ❌ Missing file: ${sourceFile}`, colors.red)
      }
    }

    // Check for extra files
    for (const languageFile of languageFiles) {
      if (!sourceFiles.includes(languageFile)) {
        issues.extraFiles.push({ language, file: languageFile })
        print(`  ❌ Extra file: ${languageFile}`, colors.red)
      }
    }

    // Validate each file
    for (const filename of sourceFiles) {
      if (!languageFiles.includes(filename)) continue // Already reported as missing

      const sourceData = loadJson(SOURCE_LANGUAGE, filename)
      const targetData = loadJson(language, filename)

      const sourceKeys = getKeys(sourceData)
      const targetKeys = getKeys(targetData)

      // Check for missing keys
      const missingKeys = sourceKeys.filter((key) => !(key in targetData))
      if (missingKeys.length > 0) {
        issues.missingKeys.push({ language, file: filename, keys: missingKeys })
        print(
          `  ❌ ${filename}: ${missingKeys.length} missing keys`,
          colors.red
        )
        missingKeys
          .slice(0, 5)
          .forEach((key) => print(`     - ${key}`, colors.red))
        if (missingKeys.length > 5) {
          print(`     ... and ${missingKeys.length - 5} more`, colors.red)
        }
      }

      // Check for extra keys
      const extraKeys = targetKeys.filter((key) => !(key in sourceData))
      if (extraKeys.length > 0) {
        issues.extraKeys.push({ language, file: filename, keys: extraKeys })
        print(`  ❌ ${filename}: ${extraKeys.length} extra keys`, colors.red)
        extraKeys
          .slice(0, 5)
          .forEach((key) => print(`     - ${key}`, colors.red))
        if (extraKeys.length > 5) {
          print(`     ... and ${extraKeys.length - 5} more`, colors.red)
        }
      }

      // Check key order (only if no missing/extra keys)
      if (missingKeys.length === 0 && extraKeys.length === 0) {
        if (!arraysEqual(sourceKeys, targetKeys)) {
          issues.wrongOrder.push({ language, file: filename })
          print(`  ❌ ${filename}: Keys in wrong order`, colors.red)
        }
      }
    }

    // Print success if no issues for this language
    const hasIssues =
      issues.missingFiles.some((i) => i.language === language) ||
      issues.extraFiles.some((i) => i.language === language) ||
      issues.missingKeys.some((i) => i.language === language) ||
      issues.extraKeys.some((i) => i.language === language) ||
      issues.wrongOrder.some((i) => i.language === language)

    if (!hasIssues) {
      print(`  ✅ All files valid`, colors.green)
    }
  }

  return issues
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

  print(`\n❌ Found ${totalIssues} issues:`, colors.bold + colors.red)

  if (issues.missingFiles.length > 0) {
    print(`   - ${issues.missingFiles.length} missing files`, colors.red)
  }
  if (issues.extraFiles.length > 0) {
    print(`   - ${issues.extraFiles.length} extra files`, colors.red)
  }
  if (issues.missingKeys.length > 0) {
    const totalMissing = issues.missingKeys.reduce(
      (sum, i) => sum + i.keys.length,
      0
    )
    print(
      `   - ${totalMissing} missing keys across ${issues.missingKeys.length} files`,
      colors.red
    )
  }
  if (issues.extraKeys.length > 0) {
    const totalExtra = issues.extraKeys.reduce(
      (sum, i) => sum + i.keys.length,
      0
    )
    print(
      `   - ${totalExtra} extra keys across ${issues.extraKeys.length} files`,
      colors.red
    )
  }
  if (issues.wrongOrder.length > 0) {
    print(
      `   - ${issues.wrongOrder.length} files with wrong key order`,
      colors.red
    )
  }

  return false
}

/**
 * Main function
 */
async function main() {
  try {
    print("\n🚀 Translation Sync Validation Script", colors.bold + colors.cyan)
    print(`I18n Path: ${I18N_PATH}`, colors.blue)

    // Parse language filter
    const languageFilter = parseLanguageFilter()
    if (languageFilter) {
      // Validate that all requested languages exist
      for (const lang of languageFilter) {
        validateLanguageExists(lang)
      }
    }

    const languages = getLanguages(languageFilter)

    // Validate
    const issues = validateTranslations(languages)
    const isValid = printSummary(issues)

    if (!isValid) {
      print(
        "\n💡 Run 'pnpm translations:sync' to fix these issues automatically\n",
        colors.yellow
      )
      process.exit(1)
    }

    print("", "") // Empty line
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
