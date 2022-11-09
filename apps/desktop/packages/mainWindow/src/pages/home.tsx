import Page from "@/components/Page";
import napi from "@/modules/napi";
import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
import { Button } from "@gd/ui";
import accounts from "@/modules/components/accounts";

export default function Home() {
  const [count, setCount] = createSignal(0);
  const [value, setValue] = createSignal<number>();

  const navigate = useNavigate();

  return (
    <Page class="bg-[#1D2028]">
      <button onClick={() => navigate("?m=myModal")}>Open modal</button>
      <button
        onClick={async () => {
          const res = await napi.auth((id) => {
            console.log("hello", id);
          });
          console.log("RES:", res);
          // setValue(res);
        }}
      >
        Auth
      </button>
    </Page>
  );
}
