import { Tabs } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Tabs",
  component: Tabs,
};

const Template = ((args) => <Tabs {...args} />) as StoryFn<
  ComponentProps<typeof Tabs>
>;

export const Main = Template.bind({});

Main.args = {
  tabs: [
    {
      name: "mods",
      component: <div>mods jsx</div>,
    },
    {
      name: "modpacks",
      component: <div>modpacks jsx</div>,
    },
  ],
};
