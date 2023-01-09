const getTranslationByLanguage = async (lang?: string) => {
  try {
    const language = await import(`./locale/${lang || "en"}/common.json`);
    return language.default;
  } catch {}
};
export { getTranslationByLanguage };
