/* eslint-disable i18next/no-literal-string */
import Card from "@/components/Card";
import { Trans } from "@gd/i18n";

const Overview = () => {
  return (
    <div class="flex flex-col gap-4 mt-10 max-w-185">
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
        <p class="m-0 text-sm leading-6">
          Minecraft Forge is a handy place to store, sort, and keep tabs on all
          your mods. If you’re after more inspiration, our guide to the best
          Minecraft shaders and Minecraft texture packs add plenty of visual
          flair to the blocky universe. Not sure what to build in game?
        </p>
      </div>
    </div>
  );
};

export default Overview;
