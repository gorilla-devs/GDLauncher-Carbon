import { getTranslationByLanguage } from "@gd/i18n";
import { Input } from "@gd/ui";
import { useI18n } from "@solid-primitives/i18n";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [t, { add, locale }] = useI18n();

  const changeLang = async (lang: string) => {
    getTranslationByLanguage(lang).then((translations) => {
      add(lang, translations);
      locale(lang);
    });
  };

  return (
    <SiderbarWrapper>
      <Input
        placeholder="Type Here"
        icon={<div class="i-ri:search-line" />}
        class="w-full rounded-full text-black-lightGray"
      />
      Sidebar library
      {t("hello")}
      {t("world")}
      <button onClick={() => changeLang("it")}>IT</button>
      <button onClick={() => changeLang("en")}>EN</button>
      <button onClick={() => changeLang("de")}>DE</button>
    </SiderbarWrapper>
  );
};

export default Sidebar;
