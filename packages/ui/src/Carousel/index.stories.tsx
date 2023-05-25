import { Carousel } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Carousel> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Carousel",
  component: Carousel,
  argTypes: {},
};

export default meta;

type Story = StoryObj<typeof Carousel>;

export const Primary: Story = {
  args: {
    title: "Recent Played",
  },
};
