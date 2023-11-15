import { Checkbox } from "./index";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Checkbox> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Checkbox",
  component: Checkbox,
};

export default meta;

type Story = StoryObj<typeof Checkbox>;

export const Checked: Story = {
  args: {
    checked: true,
    disabled: false,
  },
};
export const WithTitle: Story = {
  args: {
    checked: true,
    disabled: false,
    children: "Checkbox",
  },
};
