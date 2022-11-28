import Page from "@/components/Page";
import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
import { Button } from "@gd/ui";
import styles from "./home.module.scss";
import accounts from "@/modules/components/azureData";

export default function Home() {
  const [count, setCount] = createSignal(0);
  const [value, setValue] = createSignal<number>();

  const navigate = useNavigate();

  return (
    <Page class="bg-[#1D2028]">
      <button onClick={() => navigate("?m=myModal")}>Open modal</button>
      <button>Auth</button>
      <Button>Hello</Button>
      <div class={styles.paolo}>
        Hello
      </div>
    </Page>
  );
}
