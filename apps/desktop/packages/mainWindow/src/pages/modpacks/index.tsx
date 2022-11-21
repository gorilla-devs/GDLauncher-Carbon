import { Suspense } from "solid-js";
import { useNavigate, useRouteData } from "@solidjs/router";
import Page from "@/components/Page";

export default function About() {
  const name = useRouteData<() => string>();
  const navigate = useNavigate();

  return (
    <Page class="bg-black-black">
      <div>
        <button onClick={() => navigate("/modpacks/DDLTR")}>
          Modpack DDLTR
        </button>
      </div>
    </Page>
  );
}
