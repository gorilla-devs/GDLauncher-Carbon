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
  <div class="h-70">
    <ViewList {...args}>
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
      <div class="w-20 h-20 bg-green-500 border-dark-50" />
    </ViewList>
  </div>
)) as StoryFn<ComponentProps<typeof ViewList>>;

export const Main = Template.bind({});

Main.args = {
  cols: 8,
  type: "grid",
  itemSize: { height: 50, width: 50 },
};
