# Contributing Languages to GDLauncher

Thank you for your interest in translating GDLauncher into your language! This guide explains how to add a new language to the launcher.

## 📋 Before You Start

**Currently supported languages:**

- English (en-US)

## 🚀 Steps to Add a New Language

### 1. Generate Translation Files

From the project root, run the sync script to create language files:

```bash
pnpm translations:sync --language french
```

This command will:

- Create the language directory: `packages/i18n/locale/french/`
- Copy all English translation files as a starting point
- Mark all strings with `-------- [ UNTRANSLATED ] --------` prefix to identify what needs translation

**Note**: The same command works for both creating new languages and syncing existing outdated languages with English.

### 2. Add Language to Configuration

Edit `packages/i18n/index.ts` and add your language to the `supportedLanguages` object:

```typescript
export const supportedLanguages = {
  english: "united-states",
  french: "france" // Add your language here
}
```

**Format**: Use the language code (lowercase) as the key and map it to a country code (for flag emoji support).

### 3. Add Language Names

Edit `packages/i18n/languages.json` and add the language display name and native name:

```json
{
  "english": "English",
  "english_native": "English",
  "french": "French",
  "french_native": "Français"
}
```

**Format**:

- `language_code`: Display name in English
- `language_code_native`: Display name in the native language

### 4. Translate the Content

All generated translation files will have strings marked with the `-------- [ UNTRANSLATED ] --------` prefix:

```json
{
  "login.welcome": "-------- [ UNTRANSLATED ] --------\nWelcome to",
  "login.sign_in": "-------- [ UNTRANSLATED ] --------\nSign In",
  "instance.launch": "-------- [ UNTRANSLATED ] --------\nLaunching {{name}}"
}
```

**Translation Process:**

1. Open each JSON file in `packages/i18n/locale/french/`
2. Search for `-------- [ UNTRANSLATED ] --------` to find what needs translation
3. Replace the entire value with your translation
4. Keep keys in English (e.g., `"login.welcome"`)
5. Use your language's native punctuation and formatting
6. Preserve `{{variableName}}` placeholders (ICU message format)

**Example of translated value:**

```json
{
  "login.welcome": "Bienvenue à",
  "login.sign_in": "Se connecter",
  "instance.launch": "Lancement de {{name}}"
}
```

### 5. Validate Your Work

Ensure all your translations are valid:

```bash
pnpm translations:validate-sync --language french
```

This validates:

- All files exist in your language
- All keys match the English structure
- No extra or missing keys
- No keys left with `-------- [ UNTRANSLATED ] --------` prefix
- Proper JSON formatting

**If validation passes**, you're ready to submit! If it fails, the command will tell you exactly what's wrong.

### 6. Test in the App

1. Start the development server (see the main README for instructions)
2. Open the settings (gear icon)
3. Select "Language" settings
4. Choose your new language from the dropdown
5. Verify translations display correctly throughout the app

## Adding Missing Keys to an Existing Language

When new features are added to GDLauncher, new translation keys are added to English. To keep an existing language up to date:

### 1. Sync the Language

Run the sync script for your language:

```bash
pnpm translations:sync --language french
```

This will:
- Add any new keys from English (marked with `-------- [ UNTRANSLATED ] --------`)
- Remove deprecated keys that no longer exist
- Reorder keys to match English
- Preserve all existing translations

### 2. Translate New Keys

Search for `-------- [ UNTRANSLATED ] --------` in your language files and translate those values.

### 3. Validate

Ensure everything is correct:

```bash
pnpm translations:validate-sync --language french
```

### 4. Submit

Create a pull request with your updated translations. The maintainers will review and merge.

## 📁 Translation File Structure

The translation system is organized into 26 namespaces for better maintainability:

| Namespace     | Purpose                               |
| ------------- | ------------------------------------- |
| accounts      | Account management                    |
| addons        | Addon viewing and management          |
| ads           | Ads and revenue features              |
| app           | App updates                           |
| auth          | Login & authentication                |
| errors        | Error messages                        |
| general       | General UI elements (buttons, labels) |
| instance      | Instance/game management              |
| java          | Java runtime management               |
| library       | Library view                          |
| logs          | Logs viewer                           |
| modals        | Modal titles and messages             |
| modloaders    | Modloader names                       |
| modpacks      | Modpack information                   |
| mods          | Mod management                        |
| news          | News & changelogs                     |
| notifications | Notification messages                 |
| onboarding    | Onboarding flow                       |
| placeholders  | Input placeholders                    |
| platforms     | Platform names (CurseForge, Modrinth) |
| search        | Search functionality                  |
| settings      | Settings page options                 |
| tasks         | Task/progress status messages         |
| tracking      | Metrics tracking messages             |
| ui            | UI components                         |
| window        | Window controls                       |

## 🛠️ Useful Commands

### `pnpm translations:sync --language <name>`

Creates a new language or syncs an existing language with English:

- Creates missing translation files
- Adds missing keys (marked with `-------- [ UNTRANSLATED ] --------`)
- Removes deprecated keys
- Reorders keys to match English
- Preserves existing translations (never overwrites)

**Examples:**

```bash
pnpm translations:sync --language french        # Create or sync French
pnpm translations:sync --languages french,spanish  # Multiple languages
pnpm translations:sync                          # Sync ALL languages
```

### `pnpm translations:validate-sync --language <name>`

Validates that a language is in sync with English (without making changes):

- Checks for missing/extra files
- Checks for missing/extra keys
- Verifies key order matches English
- Reports any structural issues

Exits with code 0 if valid, 1 if issues found (suggests running `sync`).

**Examples:**

```bash
pnpm translations:validate-sync --language french  # Validate French
pnpm translations:validate-sync                    # Validate ALL languages
```

### `pnpm translations:gen-types`

Regenerates TypeScript types from translation keys for autocomplete and type checking. Run after modifying translation files.

### `pnpm translations:validate-english-keys`

Validates English translation files for correctness.

## 🌐 Flag Emoji Reference

The language selection dropdown uses flag emojis. The flag is automatically determined from the country code in `supportedLanguages`:

```typescript
export const supportedLanguages = {
  english: "united-states", // 🇺🇸
  french: "france", // 🇫🇷
  german: "germany", // 🇩🇪
  spanish: "spain", // 🇪🇸
  portuguese: "portugal", // 🇵🇹
  brazilian: "brazil" // 🇧🇷
}
```

## ❓ Questions?

If you have questions about translating a specific term or feature, feel free to open an issue or contact us on our Discord server [here](https://discord.gdlauncher.com).

Thank you for helping make GDLauncher accessible to more people! 🌍
