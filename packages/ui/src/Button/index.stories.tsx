import { Button } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Button> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Button",
  component: Button,
  argTypes: {
    type: {
      options: ["primary", "secondary", "outline", "glow"],
      control: { type: "radio" },
    },
    size: {
      options: ["large", "medium", "small"],
      control: { type: "radio" },
    },
  },
};

export default meta;

type Story = StoryObj<typeof Button>;

export const Primary: Story = {
  args: {
    disabled: false,
    variant: "primary",
    children: "Click Here",
    uppercase: true,
  },
};

export const Outline: Story = {
  args: {
    disabled: false,
    variant: "outline",
    children: "Click Here",
    uppercase: true,
  },
};

export const Icon: Story = {
  args: {
    disabled: false,
    variant: "primary",
    children: "Click Here",
    icon: <div class="i-ri:refresh-line" />,
    iconRight: false,
    uppercase: true,
  },
};
