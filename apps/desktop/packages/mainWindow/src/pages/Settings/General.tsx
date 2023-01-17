import { Dropdown, Switch } from "@gd/ui";

const General = () => {
  return (
    <div class="bg-shade-8 w-full h-full flex flex-col py-5	px-6 box-border">
      <h2 class="m-0 mb-7 text-4">General</h2>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">Language</h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            Choose a language that is convenient for you and the launcher will
            be restarted
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
        <h5 class="mt-0 mb-2">Instance Sorting</h5>
        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            Select the method in which instances should be sorted.
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
        <h5 class="mt-0 mb-2">Expert user mod</h5>

        <div class="flex w-full justify-between">
          <p class="text-shade-3 max-w-96 m-0">
            Adds more control over the settings of your instances and java.
          </p>
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">Hide launcher while playing</h5>
        <p class="text-shade-3 max-w-96 m-0">
          Automatically hide the launcher when launching an instance. You will
          still be able to open it from the icon tray.
        </p>
      </div>
      <div class="mb-6 max">
        <h5 class="mt-0 mb-2">Potato PC mode</h5>
        <p class="text-shade-3 max-w-96 m-0">
          You got a potato PC? Don't worry! We got you covered. Enable this and
          all animations and special effects will be disabled.
        </p>
      </div>
    </div>
  );
};

export default General;
