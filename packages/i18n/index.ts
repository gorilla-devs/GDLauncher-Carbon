import supportedLanguages from "./supportedLanguages.json";

type NamespacesMap = {
  [key: string]: Object;
};

const loadLanguageFiles = async (lang: string) => {
  const namespacesMap: NamespacesMap = {};
  const namespaces = ["common", "settings"];

  const language = (await import(`./locale/languages.json`)).default;
  namespacesMap["languages"] = language;

  for (const namespace of namespaces) {
    try {
      const language = (await import(`./locale/${lang}/${namespace}.json`))
        .default;

      namespacesMap[namespace] = language;
    } catch (err) {
      console.log(err);
    }
  }

  return namespacesMap;
};

export { TransProvider, Trans, useTransContext } from "@mbarzda/solid-i18next";
export { default as i18n } from "i18next";
export { default as icu } from "i18next-icu";
export { loadLanguageFiles };
export { supportedLanguages };
