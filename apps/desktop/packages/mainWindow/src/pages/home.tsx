import Page from "@/components/Page";
import { useNavigate } from "@solidjs/router";
import { Button } from "@gd/ui";
import styles from "./home.module.scss";

export default function Home() {
  const navigate = useNavigate();

  return (
    <Page class="bg-[#1D2028]">
      <button onClick={() => navigate("?m=myModal")}>Open modal</button>
      <button>Auth</button>
      <Button>Hello</Button>
      <div class={styles.paolo}>Hello</div>
    </Page>
  );
}
