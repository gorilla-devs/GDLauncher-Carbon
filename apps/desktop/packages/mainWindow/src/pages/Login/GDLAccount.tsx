import { useRouteData } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import fetchData from "./auth.login.data";

const GDLAccount = () => {
  return (
    <div class="flex flex-col h-full w-full text-center">
      <h1>GDL Account</h1>
      <p class="text-lightSlate-500 text-md">
        You can optionally create a GDL account to save your settings and
        preferences.
      </p>
      <div class="flex flex-col justify-center items-center text-center">
        GDL Account
      </div>

      <div class="flex justify-between gap-4 w-full">
        <Button type="text">
          <Trans key="login.skip" />
        </Button>
        <Button
          variant="primary"
          size="large"
          // disabled={!acceptedTOS()}
          // loading={loadingButton()}
          onClick={async () => {}}
        >
          <Trans key="login.next" />
        </Button>
      </div>
    </div>
  );
};

export default GDLAccount;
