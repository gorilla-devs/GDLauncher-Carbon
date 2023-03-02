import Button from "@/components/Button";
import Input from "@/components/Input";
import LoadingSpinner from "@/components/LoadingSpinner";
import { ADD_USER_ENDPOINT } from "@/constants";
import { useTranslations } from "@/i18n/utils";
import { createSignal, Show } from "solid-js";

const WaitList = ({ pathname }: { pathname: string }) => {
  const [email, setEmail] = createSignal("");
  const [error, setError] = createSignal<null | string>(null);
  const [loading, setLoading] = createSignal(false);
  const [success, setSuccess] = createSignal<null | string>(null);
  const t = useTranslations(pathname);

  const addUser = async (body: any) => {
    return await fetch(ADD_USER_ENDPOINT, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
  };

  const handleSubmit = async (event: Event) => {
    event.preventDefault();
    const obj: any = {};
    setError(null);
    setSuccess(null);
    setLoading(true);

    const emailRegex = /^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,4}$/i;

    if (emailRegex.test(email())) {
      if (email()) {
        obj["email"] = email();
        const res = await addUser(obj);
        if (res.status === 400) {
          setError(t("newsletter.error_400"));
        } else {
          setSuccess(t("newsletter.success"));
          setEmail("");
        }
      } else {
        setError(t("newsletter.error_missing_info"));
      }
    } else {
      setError(t("newsletter.notEmail"));
    }
    setLoading(false);
  };

  return (
    <div class="pt-10 pb-10 lg:pb-0 h-screen relative flex flex-col justify-center items-center">
      <div class="flex flex-col justify-center items-center gap-10 max-w-xs lg:max-w-5xl text-center lg:pt-24">
        <h1 class="flex flex-col lg:block text-4xl lg:text-7xl font-bold ">
          {t("newsletter.title")}
          <span class="inline-block px-4 text-yellow-500 -rotate-12 hover:rotate-0 transition-transform underline">
            {t("newsletter.title2")}
          </span>
          {t("newsletter.title3")}
        </h1>
        <p class="text-xl lg:text-3xl font-thin max-w-xs lg:max-w-4xl">
          {t("newsletter.text")}
        </p>
        <div>
          <div class="flex flex-col justify-center items-center gap-4">
            <div class="flex flex-col lg:flex-row gap-10">
              <div class="grow lg:min-w-[300px]">
                <Input
                  placeholder={t("newsletter.email")}
                  type="email"
                  value={email()}
                  onInput={(e) => {
                    const value = (e.target as HTMLInputElement).value;
                    setEmail(value);
                  }}
                />
              </div>
              <Button
                onClick={handleSubmit}
                disabled={loading() || !!success()}
                class="min-w-[100px] border-none transition duration-150 box-shadow-button hover:box-shadow-button-hover active:box-shadow-button-active"
              >
                <>
                  <Show when={loading()}>
                    <LoadingSpinner />
                  </Show>
                  {!!success()
                    ? t("newsletter.subscribed")
                    : t("newsletter.getAccess")}
                </>
              </Button>
            </div>
            <div class=" lg:h-[100px]">
              <Show when={error()}>
                <div class="p-10 text-red-400">{error()}</div>
              </Show>
              <Show when={success()}>
                <div class="p-10 text-green-400">{success()}</div>
              </Show>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default WaitList;
