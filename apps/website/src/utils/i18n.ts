import i18next from "i18next";
import HttpApi from "i18next-http-backend";
import { SUPPORTED_LANGUAGES } from "@/constants";

export const init = async (namespaces: string[] = []) => {
  if (globalThis.document) {
    // Get the language that has been set by SSR in the html attribute
    const lng = globalThis.document
      ?.querySelector("html")
      ?.getAttribute?.("lang");

    await i18next.use(HttpApi).init({
      debug: false,
      backend: {
        loadPath: "/intl/{{lng}}/{{ns}}.json",
      },
      supportedLngs: SUPPORTED_LANGUAGES,
      lng: lng || "en",
      ns: ["common", ...namespaces],
      defaultNS: "common",
      load: "languageOnly",
      interpolation: {
        escapeValue: false,
      },
    });
  }
};

// TODO: Force usage of these functions over the default/astro i18next ones.
type Format = "capitalize" | "lowercase" | "uppercase";
export const format = (value: string, formatType: Format) => {
  if (!value) return "";
  switch (formatType) {
    case "capitalize":
      return value.charAt(0).toUpperCase() + value.slice(1);
    case "lowercase":
      return value.toLowerCase();
    case "uppercase":
      return value.toUpperCase();
    default:
      return value;
  }
};
