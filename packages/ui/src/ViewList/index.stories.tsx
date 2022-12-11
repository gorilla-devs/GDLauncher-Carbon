import { ViewList } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "ViewList",
  component: ViewList,
  argTypes: {
    type: {
      options: ["list", "grid"],
      control: { type: "radio" },
    },
  },
};

const Template = ((args) => (
  <ViewList {...args}>
    <div class="w-20 h-20 bg-green-500" />
    <div class="w-20 h-20 bg-green-500" />
    <div class="w-20 h-20 bg-green-500" />
  </ViewList>
)) as StoryFn<ComponentProps<typeof ViewList>>;

export const Main = Template.bind({});

Main.args = {
  cols: 4,
  type: "grid",
};
