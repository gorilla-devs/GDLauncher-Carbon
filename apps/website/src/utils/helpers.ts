export function getLanguageFromURL(pathname: string) {
  const langCodeMatch = pathname.match(/\/([a-z]{2}-?[a-z]{0,2})\//);
  return langCodeMatch ? langCodeMatch[1] : "en";
}

export const localizePath = (
  path: string,
  lang: string,
  isExternalLink?: boolean
) => {
  if (isExternalLink) return path;

  return `/${lang}${path}`;
};
