import { Slider } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Slider> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Slider",
  component: Slider,
};

export default meta;

type Story = StoryObj<typeof Slider>;

export const Main: Story = {
  args: {
    min: 0,
    max: 100,
    onChange: (val) => console.log("onChange", val),
  },
};
