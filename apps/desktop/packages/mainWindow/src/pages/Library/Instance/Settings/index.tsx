import { Trans } from "@gd/i18n";
import { Slider } from "@gd/ui";

const Settings = () => {
  return (
    <div>
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="instance_settings_title"
          options={{
            defaultValue: "Settings",
          }}
        />
      </h2>
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
  );
};

export default Settings;
