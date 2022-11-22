import { Button } from "./index.jsx";
import type { Meta, StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/html/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Button",
  component: Button,
  argTypes: {
    type: {
      options: ["primary", "secondary", "outline"],
      control: { type: "radio" },
    },
  },
};

const Template = ((args) => <Button {...args} />) as StoryFn<
  ComponentProps<typeof Button>
>;

export const Disabled = Template.bind({});

Disabled.args = {
  disabled: true,
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
