import { Progressbar } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Progressbar> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Progressbar",
  component: Progressbar,
};

export default meta;

type Story = StoryObj<typeof Progressbar>;

export const Main: Story = {
  args: {
    percentage: 0,
  },
};
