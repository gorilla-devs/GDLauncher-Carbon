import Button from "@/components/Button";
import Input from "@/components/Input";
import { ADD_USER_ENDPOINT } from "@/constants";
import { useTranslations } from "@/i18n/utils";
import { createSignal, Show } from "solid-js";
import { useForm } from "../../../components/useForm";
interface IForm {
  email: string;
  emailMc: string;
  emailKofi: string;
}

const WaitList = ({ pathname }: { pathname: string }) => {
  const { form, updateFormField } = useForm();
  const [error, setError] = createSignal("");
  const t = useTranslations(pathname);

  const addUser = async (body: any) =>
    await fetch(ADD_USER_ENDPOINT, {
      method: "POST",
      body,
    });

  const handleSubmit = async (event: Event) => {
    event.preventDefault();
    const formData = new FormData();

    if (form.email && form.emailMc && form.emailKofi) {
      formData.append("user_email", form.email);
      formData.append("mc_email", form.emailMc);
      formData.append("kofi_email", form.emailKofi);

      const res = await addUser(formData);
      console.log("res", res);
      if (res.status === 422) {
        setError(t("waitlist.error_422"));
      }
    }
  };

  return (
    <div class="h-[calc(100vh-5.5rem)] lg:h-screen relative flex flex-col justify-center items-center">
      <div class="flex flex-col justify-center items-center gap-10 max-w-xs lg:max-w-5xl text-center lg:pt-24">
        <h1 class="flex flex-col lg:block text-4xl lg:text-7xl font-bold ">
          {t("waitlist.title")}
          <span class="inline-block px-4 text-yellow-500 -rotate-10 hover:rotate-0 transition-transform underline">
            {t("waitlist.title2")}
          </span>
          {t("waitlist.title3")}
        </h1>
        <p class="text-xl lg:text-3xl font-thin max-w-xs lg:max-w-4xl">
          {t("waitlist.text")}
        </p>
        <form onSubmit={handleSubmit} method="post">
          <div class="flex flex-col justify-center items-center gap-10">
            <div class="w-full">
              <Input
                placeholder={t("waitlist.email")}
                type="email"
                value={form.email}
                onChange={updateFormField("email")}
              />
            </div>
            <div class="flex flex-col lg:flex-row gap-10">
              <Input
                placeholder={t("waitlist.mc_email")}
                type="email"
                value={form.emailMc}
                onChange={updateFormField("emailMc")}
              />
              <Input
                placeholder={t("waitlist.kofi_email")}
                type="email"
                value={form.emailKofi}
                onChange={updateFormField("emailKofi")}
              />
            </div>
            <Button class="min-w-[260px] border-none transition duration-150 box-shadow-button hover:box-shadow-button-hover active:box-shadow-button-active">
              {t("waitlist.getAccess")}
            </Button>
            <Show when={error()}>
              <div class="p-10 text-red-400">{error()}</div>
            </Show>
          </div>
        </form>
      </div>
    </div>
  );
};

export default WaitList;
