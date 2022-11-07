import Page from "@/components/Page";
import napi from "@/utils/napi";
import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
import { Button } from "@gd/ui";

export default function Home() {
  const [count, setCount] = createSignal(0);
  const [value, setValue] = createSignal<number>();

  const navigate = useNavigate();

  return (
    <Page>
      <div class="flex justify-center items-center w-full h-full">
        <button
          onClick={() => {
            navigate("/home");
          }}
        >
          Auth
        </button>
      </div>
    </Page>
  );
}
