import type { Translation } from "@gd/core_module/bindings"

/** Interpolation options for a backend task title. The arguments are mod,
 *  instance, server and world names, and every caller renders the result as
 *  text, so i18next must not HTML-escape them ("Farmer&#39;s Delight"). */
export function taskTranslationOptions(translation: Translation) {
  return {
    ...("args" in translation ? translation.args : {}),
    interpolation: { escapeValue: false }
  }
}
