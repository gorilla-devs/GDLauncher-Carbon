import { Switch } from "./index";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Switch",
  component: Switch,
  argTypes: {},
};

const Template = ((args) => <Switch {...args} />) as StoryFn<
  ComponentProps<typeof Switch>
>;

export const Checked = Template.bind({});

Checked.args = {
  disabled: false,
  checked: true,
};

export const UnChecked = Template.bind({});

UnChecked.args = {
  disabled: false,
  checked: false,
};
