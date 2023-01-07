import { useLanguages } from "@/languagesProvider";
import { Input } from "@gd/ui";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  const [t, changeLang] = useLanguages();
  console.log("useLanguages", t);
  return (
    <SiderbarWrapper>
      <Input
        placeholder="Type Here"
        icon={<div class="i-ri:search-line" />}
        class="w-full rounded-full text-black-lightGray"
      />
      Sidebar library
      {t("hello")}
      <button onClick={() => changeLang("it")}>IT</button>
      <button onClick={() => changeLang("en")}>EN</button>
    </SiderbarWrapper>
  );
};

export default Sidebar;
