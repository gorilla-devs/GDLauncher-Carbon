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
    checked: true,
  },
};

export const CheckedDisabled: Story = {
  args: {
    checked: true,
    disabled: true,
  },
};

export const UnChecked: Story = {
  args: {
    checked: false,
  },
};

export const UnCheckedDisabled: Story = {
  args: {
    checked: false,
    disabled: true,
  },
};

export const Loading: Story = {
  args: {
    isLoading: true,
  },
};

export const Indeterminate: Story = {
  args: {
    isIndeterminate: true,
  },
};
