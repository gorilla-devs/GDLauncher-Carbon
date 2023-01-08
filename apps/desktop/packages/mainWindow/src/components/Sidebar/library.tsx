import { useLanguages } from "@/languagesProvider";
import { Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [t, changeLang] = useLanguages();

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
