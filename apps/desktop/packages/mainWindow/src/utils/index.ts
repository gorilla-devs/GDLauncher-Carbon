export const getBasePathUrl = (baseUrl: string) => {
  const removeLastSection = (url: string) => {
    if (url.endsWith("/")) {
      url = url.slice(0, -1);
    }

    let sections = url.split("/");

    return sections.slice(0, sections.length - 1).join("/");
  };

  let basePath =
    "file://" +
    removeLastSection(baseUrl.split("app.asar")[0]).replace("file://", "");

  return basePath;
};
