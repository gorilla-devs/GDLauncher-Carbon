import supportedLanguages from "./supportedLanguages.json";

const loadLanguageFile = async (lang: string) => {
  try {
    const language = await import(`./locale/${lang}/common.json`);
    return language.default;
  } catch (err) {
    console.error("Language file not found", err);
  }
};

const loadLanguagesFile = async () => {
  try {
    const language = await import(`./locale/languages.json`);
    return language.default;
  } catch (err) {
    console.error("Language file not found", err);
  }
};

export { TransProvider, Trans, useTransContext } from "@mbarzda/solid-i18next";
export { default as i18n } from "i18next";
export { default as icu } from "i18next-icu";
export { loadLanguageFile, loadLanguagesFile };
export { supportedLanguages };
