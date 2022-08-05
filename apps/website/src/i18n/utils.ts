import { getLanguageFromURL } from "@/utils/helpers";
import { AstroGlobal } from "astro";
import enUI from "@/i18n/en/ui";

export type UIDictionaryKeys = keyof typeof enUI;
export type UIDict = Partial<typeof enUI>;
export const UIDictionary = (dict: Partial<typeof enUI>) => dict;

const fallbackLang = "en";

/**
 * Convert the map of modules returned by `import.meta.globEager` to an object
 * mapping the language code from each module’s filepath to the module’s default export.
 */
function mapDefaultExports<T>(modules: Record<string, { default: T }>) {
  const exportMap: Record<string, T> = {};
  for (const [path, module] of Object.entries(modules)) {
    const [_dot, lang] = path.split("/");
    exportMap[lang] = module.default;
  }
  return exportMap;
}

const translations = mapDefaultExports<UIDict>(
  import.meta.globEager("./*/ui.ts")
);

export function useTranslations(
    Astro: Readonly<AstroGlobal>
  ): (key: UIDictionaryKeys) => string | undefined {
    const lang = getLanguageFromURL(Astro.url.pathname) || "en";
    return function getTranslation(key: UIDictionaryKeys) {
      const str =
        translations[lang]?.[key] || translations[fallbackLang][key];
      if (str === undefined)
        console.error(`Missing translation for “${key}” in “${lang}”.`);
      return str;
      // return "";
    };
  }
  