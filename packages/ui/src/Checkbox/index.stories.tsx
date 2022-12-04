import { Checkbox } from "./index";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Checkbox",
  component: Checkbox,
};

const Template = ((args) => <Checkbox {...args} />) as StoryFn<
  ComponentProps<typeof Checkbox>
>;

export const Checked = Template.bind({});

Checked.args = {
  checked: true,
};
