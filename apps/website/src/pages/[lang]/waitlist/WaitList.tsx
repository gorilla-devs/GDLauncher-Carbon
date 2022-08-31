import Button from "@/components/Button";
import Input from "@/components/Input";
import { useTranslations } from "@/i18n/utils";
import html from "solid-js/html";

const WaitList = ({ pathname }: { pathname: string }) => {
  const t = useTranslations(pathname);
  return (
    <div class="h-[calc(100vh-1.5rem)] lg:h-screen relative flex flex-col justify-center items-center">
      <div class="flex flex-col justify-center items-center gap-10 max-w-5xl text-center">
        <h1 class="flex flex-col lg:block text-2xl lg:text-7xl font-bold ">
          {t("waitlist.title")}
          <span class="inline-block px-4 text-yellow-500 -rotate-10 hover:-rotate-25 transition-transform underline">
            {t("waitlist.title2")}
          </span>
          {t("waitlist.title3")}
        </h1>
        <div class="flex flex-col justify-center items-center gap-10">
          <div class="flex flex-col lg:flex-row gap-10">
            <Input placeholder={t("waitlist.email")} type="email" />
            <Input placeholder={t("waitlist.mc_email")} type="email" />
          </div>
          <div class="w-full">
            <Input placeholder={t("waitlist.kofi_email")} type="email" />
          </div>
          <Button>{t("waitlist.getAccess")}</Button>
        </div>
      </div>
    </div>
  );
};

export default WaitList;
