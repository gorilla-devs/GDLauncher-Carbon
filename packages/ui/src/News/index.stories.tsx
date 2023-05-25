import { News } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof News> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "News",
  component: News,
};

export default meta;

type Story = StoryObj<typeof News>;

export const Main: Story = {
  args: {
    rtl: true,
    disableAutoRotation: false,
    showArrows: true,
    showIndicators: true,
    slides: [
      {
        title: "title",
        description: "this is a nice and fair description",
        image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
        url: "https://randomurl.com",
      },
      {
        title: "title1",
        description: "this is a nice and fair description",
        image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
        url: "https://randomurl.com",
      },
      {
        title: "title2",
        description: "this is a nice and fair description",
        image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
        url: "https://randomurl.com",
      },
      {
        title: "title3",
        description: "this is a nice and fair description",
        image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
        url: "https://randomurl.com",
      },
    ],
  },
};
