import { Show, createEffect, createSignal } from "solid-js";
import Button from "./button/Button";
import { ADD_USER_ENDPOINT } from "../consts";
import { ImSpinner9 } from "solid-icons/im";

const NewsLetterAction = () => {
  const [email, setEmail] = createSignal("");
  const [error, setError] = createSignal<null | string>(null);
  const [loading, setLoading] = createSignal(false);
  const [success, setSuccess] = createSignal<null | string>(null);
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
    // event.preventDefault();
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
          setError(
            "You already subscribed, one of your email was already used!",
          );
        } else {
          setSuccess(
            "We have sent you a confirmation email, please click on the confirmation link. 🚀",
          );
          setEmail("");
        }
      } else {
        setError("Please enter all the info, some of the them are missing.");
      }
    } else {
      setError("Please enter a correct email.");
    }
    setLoading(false);
  };

  return (
    <div class="flex gap-2 flex-col justify-center">
      <div class="flex flex-col md:flex-row gap-2 mx-auto">
        <input
          type="email"
          placeholder="Email"
          value={email()}
          onInput={(e) => {
            const value = (e.target as HTMLInputElement).value;
            setEmail(value);
          }}
          id="newsletter-email"
          name="email"
          class="p-3 text-black rounded-xsgd outline-none"
        />
        <Button
          intent="primary"
          size="small"
          onClick={handleSubmit as any}
          disabled={loading()}
          class={`flex items-center justify-center rounded-xssgd ${loading() ? "w-6" : ""}`}
        >
          <Show when={loading()}>
            <ImSpinner9 class="animate-spin" />
          </Show>
          <Show when={!loading()}>
            <span class="font-semibold">SUBSCRIBE TO NEWSLETTER</span>
          </Show>
        </Button>
      </div>
      <Show when={error()}>
        <div class="text-red-400 font-semibold">{error()}</div>
      </Show>
      <Show when={success()}>
        <div class="text-green-400 font-semibold">{success()}</div>
      </Show>
    </div>
  );
};
export default NewsLetterAction;
