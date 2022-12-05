import { Input } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Input",
  component: Input,
};

const Template = ((args) => <Input {...args} />) as StoryFn<
  ComponentProps<typeof Input>
>;

export const Main = Template.bind({});

Main.args = {
  value: "Value",
  placeholder: "Type here",
  error: "",
  disabled: false,
  icon: <div class="i-ri:refresh-line" />,
};
