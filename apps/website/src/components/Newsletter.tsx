import { Show, createSignal } from "solid-js";
import Button from "./button/Button";
import { ADD_USER_ENDPOINT } from "../consts";

export interface NewsletterLabels {
  heading: string;
  emailPlaceholder: string;
  subscribe: string;
  errorAlreadySubscribed: string;
  errorMissingInfo: string;
  errorInvalidEmail: string;
  /** Generic failure shown on 5xx / network errors. Optional for backward
   *  compatibility; falls back to errorMissingInfo if not provided. */
  errorGeneric?: string;
  success: string;
}

interface Props {
  labels: NewsletterLabels;
}

const NewsLetter = (props: Props) => {
  const [email, setEmail] = createSignal("");
  const [error, setError] = createSignal<null | string>(null);
  const [loading, setLoading] = createSignal(false);
  const [success, setSuccess] = createSignal<null | string>(null);

  const addUser = (body: { email: string }) =>
    fetch(ADD_USER_ENDPOINT, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });

  const handleSubmit = async (event?: Event) => {
    event?.preventDefault();
    if (loading()) return;

    const value = email().trim();
    setError(null);
    setSuccess(null);

    if (!value) {
      setError(props.labels.errorMissingInfo);
      return;
    }
    // Trust the browser's built-in email validity: covers long TLDs (.museum,
    // .software) that the previous homegrown regex rejected.
    const input = document.getElementById(
      "newsletter-email",
    ) as HTMLInputElement | null;
    if (input && !input.checkValidity()) {
      setError(props.labels.errorInvalidEmail);
      return;
    }

    setLoading(true);
    try {
      const res = await addUser({ email: value });
      if (res.ok) {
        setSuccess(props.labels.success);
        setEmail("");
      } else if (res.status === 400 || res.status === 409) {
        setError(props.labels.errorAlreadySubscribed);
      } else {
        setError(
          props.labels.errorGeneric ?? props.labels.errorMissingInfo,
        );
      }
    } catch {
      setError(props.labels.errorGeneric ?? props.labels.errorMissingInfo);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      class="flex flex-col gap-2 flex-1 items-start justify-start"
      id="newsletter"
      onSubmit={handleSubmit}
      noValidate
    >
      <label
        for="newsletter-email"
        class="text-white font-medium pb-4"
      >
        {props.labels.heading}
      </label>
      <input
        type="email"
        placeholder={props.labels.emailPlaceholder}
        value={email()}
        onInput={(e) => setEmail(e.currentTarget.value)}
        id="newsletter-email"
        name="email"
        required
        autocomplete="email"
        aria-invalid={error() ? "true" : "false"}
        aria-describedby="newsletter-feedback"
        class="bg-darkgd px-2 py-1 border-bluegd-500 border-[1px] rounded-md text-white outline-none"
      />
      <div id="newsletter-feedback" class="min-h-[1.25rem]">
        <Show when={error()}>
          <div class="text-red-400" role="alert">
            {error()}
          </div>
        </Show>
        <Show when={success()}>
          <div class="text-green-400" role="status" aria-live="polite">
            {success()}
          </div>
        </Show>
      </div>
      <Button
        type="submit"
        intent="primary"
        size="small"
        disabled={loading()}
        aria-busy={loading() ? "true" : "false"}
        class="mt-3 rounded-xsgd uppercase"
      >
        {props.labels.subscribe}
      </Button>
    </form>
  );
};
export default NewsLetter;
