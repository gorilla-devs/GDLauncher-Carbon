/* eslint-disable i18next/no-literal-string */
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Input, Slider, Switch } from "@gd/ui";

const Settings = () => {
  return (
    <div class="pt-10">
      <div class="mb-6">
        <div class="w-full flex justify-between items-center mb-4">
          <h5 class="m-0">
            <Trans
              key="java_memory_title"
              options={{
                defaultValue: "Java Memory",
              }}
            />
          </h5>
          <Switch checked={true} />
        </div>
        <div class="flex justify-center px-2">
          <Slider
            min={0}
            max={16384}
            steps={1}
            value={4096}
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
        <div class="w-full flex justify-between items-center mb-4">
          <h5 class="m-0">
            <Trans
              key="game_resolution"
              options={{
                defaultValue: "Game Resolution",
              }}
            />
          </h5>
          <Switch checked={true} />
        </div>
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
      <div class="mb-6">
        <div class="w-full flex justify-between items-center mb-4">
          <h5 class="m-0">
            <Trans
              key="java_arguments_title"
              options={{
                defaultValue: "Java Arguments",
              }}
            />
          </h5>
          <Switch checked={true} />
        </div>
        <div class="flex w-full gap-4 items-center">
          <Input class="w-full" />
          <Button rounded={false} variant="secondary" class="h-10">
            <Trans
              key="reset_java_args"
              options={{
                defaultValue: "Reset",
              }}
            />
          </Button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
