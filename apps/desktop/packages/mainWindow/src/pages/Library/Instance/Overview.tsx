import Card from "@/components/Card";
import { Trans } from "@gd/i18n";
import { Show } from "solid-js";
import fetchData from "./instance.data";
import { useRouteData } from "@solidjs/router";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  return (
    <div class="flex flex-col gap-4 max-w-185 mt-10">
      <div class="w-full flex justify-center flex-wrap gap-4">
        <Card title="Minecraft version" text="1.19.2" icon="vanilla" />
        <Card title="Minecraft version" text="1.19.2" icon="book" />
        <Card title="Minecraft version" text="1.19.2" icon="pickaxe" />
        <Card title="Minecraft version" text="1.19.2" icon="cart" />
        <Card title="Minecraft version" text="1.19.2" icon="clock" />
        <Card title="Minecraft version" text="1.19.2" icon="sign" />
      </div>
      <div class="flex flex-col items-start justify-between gap-2 p-5 bg-darkSlate-700 rounded-xl box-border w-full w-59">
        <div class="text-darkSlate-50 uppercase">
          <Trans
            key="instance.notes"
            options={{
              defaultValue: "notes",
            }}
          />
        </div>
        <Show when={routeData.instanceDetails.data?.notes}>
          <p class="m-0 text-sm leading-6">
            {routeData.instanceDetails.data?.notes}
          </p>
        </Show>
      </div>
    </div>
  );
};

export default Overview;
