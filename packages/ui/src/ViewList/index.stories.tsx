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

const Template = ((args) => <ViewList {...args} />) as StoryFn<
  ComponentProps<typeof ViewList>
>;

export const Main = Template.bind({});

Main.args = {
  list: [
    { name: "name" },
    { name: "name" },
    { name: "name" },
    { name: "name" },
  ],
  cols: 4,
  type: "grid",
};
