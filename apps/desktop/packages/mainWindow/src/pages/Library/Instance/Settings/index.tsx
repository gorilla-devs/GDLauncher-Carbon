/* eslint-disable i18next/no-literal-string */
import { Trans } from "@gd/i18n";
import { Dropdown, Input, Slider } from "@gd/ui";

const Settings = () => {
  return (
    <div class="pt-10">
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="instance_settings_title"
          options={{
            defaultValue: "Settings",
          }}
        />
      </h2>
      <div class="mb-6">
        <h5 class="m-0 mb-4">
          <Trans
            key="java_memory_title"
            options={{
              defaultValue: "Java Memory",
            }}
          />
        </h5>
        <div class="flex justify-center">
          <Slider
            min={0}
            max={16384}
            steps={1}
            marks={{
              1024: "1024 MB",
              2048: "2048 MB",
              4096: "4096 MB",
              8192: "8192 MB",
              16384: "16384 MB",
            }}
          />
        </div>
      </div>

      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="game_resolution"
            options={{
              defaultValue: "Game Resolution",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <div class="flex gap-4 items-center">
            <Input class="w-20" placeholder="width" value={"1024"} />
            x
            <Input class="w-20" placeholder="height" value={"768"} />
          </div>
          <Dropdown
            value="en"
            placeholder="presets"
            options={[
              { label: "800x600", key: "800x600" },
              { label: "1024x768", key: "1024x768" },
              { label: "1920x1080", key: "1920x1080" },
            ]}
          />
        </div>
      </div>
    </div>
  );
};

export default Settings;
