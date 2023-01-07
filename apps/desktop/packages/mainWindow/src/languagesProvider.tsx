import { getTranslationByLanguage } from "@gd/i18n";
import {
  createSignal,
  createContext,
  useContext,
  createEffect,
} from "solid-js";
import { JSX } from "solid-js/jsx-runtime";
import { createStore } from "solid-js/store";

const languagesContext = createContext();

interface Translations {
  [key: string]: string;
}
interface LanguagesHashMap {
  [key: string]: Translations;
}

interface Props {
  children: HTMLElement | JSX.Element;
}
const [currentLang, setCurrentLang] = createSignal("en");

export const LanguagesProvider = (props: Props) => {
  const [languages, setLanguages] = createStore<LanguagesHashMap>({});

  createEffect(() => {
    if (languages[currentLang()]) {
      setLanguages(currentLang(), (tr) => tr);
    } else {
      getTranslationByLanguage(currentLang()).then(
        (translations: Translations) => {
          setLanguages(currentLang(), translations);
        }
      );
    }
  });
  const lang = () => languages[currentLang()] || {};

  const value = () => [
    (key: string) => {
      return lang()[key] || key;
    },
    (lang: string) => {
      setCurrentLang(lang);
    },
  ];

  return (
    <languagesContext.Provider value={value()}>
      <div>{props.children}</div>
    </languagesContext.Provider>
  );
};

export const useLanguages = () => {
  return useContext(languagesContext);
};

export default {};
