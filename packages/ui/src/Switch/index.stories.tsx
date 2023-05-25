import { Switch } from "./index";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Switch> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Switch",
  component: Switch,
};

export default meta;

type Story = StoryObj<typeof Switch>;

export const Checked: Story = {
  args: {
    disabled: false,
    checked: true,
  },
};
export const UnChecked: Story = {
  args: {
    disabled: false,
    checked: false,
  },
};
