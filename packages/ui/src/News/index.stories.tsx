import { News } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "News",
  component: News,
};

const Template = ((args) => <News {...args} />) as StoryFn<
  ComponentProps<typeof News>
>;

export const Main = Template.bind({});

Main.args = {
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
};
