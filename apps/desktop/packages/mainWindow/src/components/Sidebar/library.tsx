/* eslint-disable i18next/no-literal-string */
import { loadLanguageFile, useTransContext, Trans } from "@gd/i18n";
import { createNotification, Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [addNotification] = createNotification();
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
        class="w-full rounded-full text-shade-0"
      />
      {/* {t("hello")} */}
      <Trans
        key="hello"
        options={{
          defaultValue: "Hello",
        }}
      />
      {t("world")}
      <button onClick={() => changeLang("it")}>IT</button>
      <button onClick={() => changeLang("en")}>EN</button>
      <button onClick={() => changeLang("de")}>DE</button>
      <button onClick={() => addNotification("Notification Added")}>
        Add Notification
      </button>
    </SiderbarWrapper>
  );
};

export default Sidebar;
