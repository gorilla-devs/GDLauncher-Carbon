import { Button, Dropdown, Switch } from "@gd/ui";
import GDLauncherWideLogo from "/assets/images/gdlauncher_logo.svg";
import GDLauncherText from "/assets/images/GDLauncher_text.svg";
import { Trans } from "@gd/i18n";

const General = () => {
  return (
    <div class="bg-shade-8 w-full h-auto flex flex-col py-5 px-6 box-border">
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="general"
          options={{
            defaultValue: "General",
          }}
        />
      </h2>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="language"
            options={{
              defaultValue: "Language",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 m-0 max-w-96">
            <Trans
              key="choose_a_language"
              options={{
                defaultValue:
                  "Choose a language that is convenient for you and the launcher will be restarted",
              }}
            />
          </p>
          <Dropdown
            value="en"
            options={[
              { label: "english", key: "eng" },
              { label: "italian", key: "it" },
            ]}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="instance_sorting"
            options={{
              defaultValue: "Instance Sorting",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            <Trans
              key="select_instance_sorting_method"
              options={{
                defaultValue:
                  "Select the method in which instances should be sorted.",
              }}
            />
          </p>
          <Dropdown
            value="en"
            options={[
              { label: "Alphabetical", key: "alphabetical" },
              { label: "creation", key: "creation" },
            ]}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="expert_user_mod"
            options={{
              defaultValue: "Expert user mod",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            <Trans
              key="settings_more_control"
              options={{
                defaultValue:
                  "Adds more control over the settings of your instances and java.",
              }}
            />
          </p>
          <Switch checked={true} />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings_hide_launcher_playing_title"
            options={{
              defaultValue: "Hide launcher while playing",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            <Trans
              key="settings_hide_launcher_playing_text"
              options={{
                defaultValue:
                  "Automatically hide the launcher when launching an instance. You will still be able to open it from the icon tray.",
              }}
            />
          </p>
          <Switch checked={false} />
        </div>
      </div>
      <div class="mb-6 max">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings_potatp_mode_title"
            options={{
              defaultValue: "Potato PC mode",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            <Trans
              key="settings_potatp_mode_text"
              options={{
                defaultValue:
                  "You got a potato PC? Don't worry! We got you covered. Enable this and all animations and special effects will be disabled.",
              }}
            />
          </p>
          <Switch checked={false} />
        </div>
      </div>
      <Button rounded={false} variant="secondary" textColor="text-red">
        <Trans
          key="settings_reset"
          options={{
            defaultValue: "Reset all to default",
          }}
        />
      </Button>
      <div class="flex mb-6 mt-16 gap-24">
        <div class="flex gap-5">
          <img src={GDLauncherWideLogo} class="h-14 cursor-pointer" />
          <div class="flex flex-col">
            <img src={GDLauncherText} class="cursor-pointer h-5" />
            <p class="mb-0 mt-2">v.1.1.26</p>
          </div>
        </div>
        <p class="text-shade-5 m-0">
          <Trans
            key="settings_last_version"
            options={{
              defaultValue:
                "You're currently on the latest version. We automatically check for updates and we will inform you whenever one is available.",
            }}
          />
        </p>
      </div>
    </div>
  );
};

export default General;
