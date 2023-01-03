import { Slider } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Slider",
  component: Slider,
};

const Template = ((args) => <Slider {...args} />) as StoryFn<
  ComponentProps<typeof Slider>
>;

export const Main = Template.bind({});

Main.args = {
  min: 0,
  max: 100,
  onChange: (val) => console.log("onChange", val),
};
