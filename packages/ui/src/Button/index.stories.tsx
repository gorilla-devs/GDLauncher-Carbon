import { Button } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Button",
  component: Button,
  argTypes: {
    type: {
      options: ["primary", "secondary", "outline", "glow"],
      control: { type: "radio" },
    },
  },
};

const Template = ((args) => <Button {...args} />) as StoryFn<
  ComponentProps<typeof Button>
>;

export const Primary = Template.bind({});

Primary.args = {
  disabled: false,
  type: "primary",
  children: "Click Here",
};

export const Outline = Template.bind({});

Outline.args = {
  disabled: false,
  type: "outline",
  children: "Click Here",
};

export const Icon = Template.bind({});

Icon.args = {
  disabled: false,
  type: "primary",
  children: "Click Here",
  icon: <div class="i-ri:refresh-line" />,
  iconRight: false,
};
