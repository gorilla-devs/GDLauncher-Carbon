import { createSignal } from "solid-js";
import Button from "./button/Button";

const NewsLetter = () => {
  const [email, setEmail] = createSignal("");
  return (
    <div class="flex flex-col gap-2">
      <span class="text-white font-medium pb-4">Newsletter</span>
      <input
        type="email"
        placeholder="Email"
        value={email()}
        onInput={(e) => {
          const value = (e.target as HTMLInputElement).value;
          setEmail(value);
        }}
        class="bg-darkgd px-2 py-1 border-bluegd-500 border-[1px] rounded-md text-white outline-none"
      />
      <Button intent="primary" size="small">
        SUBSCRIBE TO NEWSLETTER
      </Button>
    </div>
  );
};
export default NewsLetter;
