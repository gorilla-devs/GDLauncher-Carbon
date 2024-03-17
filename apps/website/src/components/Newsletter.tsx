import { Show, createSignal } from "solid-js";
import Button from "./button/Button";
import { ADD_USER_ENDPOINT } from "../consts";

const NewsLetter = () => {
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
    <div class="flex flex-col gap-2 flex-1 items-start justify-start">
      <span class="text-white font-medium pb-4">Newsletter</span>
      <input
        type="email"
        placeholder="Email"
        value={email()}
        onInput={(e) => {
          const value = (e.target as HTMLInputElement).value;
          setEmail(value);
        }}
        name="email"
        class="bg-darkgd px-2 py-1 border-bluegd-500 border-[1px] rounded-md text-white outline-none"
      />
      <Show when={error()}>
        <div class="text-red-400">{error()}</div>
      </Show>
      <Show when={success()}>
        <div class="text-green-400">{success()}</div>
      </Show>
      <Button
        intent="primary"
        size="small"
        onClick={handleSubmit as any}
        class="mt-3 rounded-xsgd"
      >
        SUBSCRIBE TO NEWSLETTER
      </Button>
    </div>
  );
};
export default NewsLetter;
