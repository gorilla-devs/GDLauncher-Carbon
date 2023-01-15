import { loadLanguageFile, useTransContext } from "@gd/i18n";
import { Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [t, { changeLanguage, addResources }] = useTransContext();

  const changeLang = async (lang: string) => {
    const langFile = await loadLanguageFile(lang);
    addResources(lang, "common", langFile);
    await changeLanguage(lang);
  };

  return (
    <SiderbarWrapper>
      <Input
        placeholder="Type Here"
        icon={<div class="i-ri:search-line" />}
        class="w-full rounded-full text-black-lightGray"
      />
      {t("hello")}
      {t("world")}
      <button onClick={() => changeLang("it")}>IT</button>
      <button onClick={() => changeLang("en")}>EN</button>
      <button onClick={() => changeLang("de")}>DE</button>
    </SiderbarWrapper>
  );
};

export default Sidebar;
