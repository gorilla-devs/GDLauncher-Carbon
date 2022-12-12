import { Progressbar } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Progressbar",
  component: Progressbar,
};

const Template = ((args) => <Progressbar {...args} />) as StoryFn<
  ComponentProps<typeof Progressbar>
>;

export const Main = Template.bind({});

Main.args = {
  percentage: 0,
};
