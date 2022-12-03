import { Carousel } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Carousel",
  component: Carousel,
  argTypes: {
    type: {
      options: ["primary", "secondary", "outline"],
      control: { type: "radio" },
    },
  },
};

const Template = ((args) => <Carousel {...args} />) as StoryFn<
  ComponentProps<typeof Carousel>
>;

export const Main = Template.bind({});

Main.args = {
  title: "Recent Played",
};
