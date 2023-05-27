import { Input } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Input> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Input",
  component: Input,
};

export default meta;

type Story = StoryObj<typeof Input>;

export const Main: Story = {
  args: {
    value: "Value",
    placeholder: "Type here",
    error: "",
    disabled: false,
    icon: <div class="i-ri:refresh-line" />,
  },
};
