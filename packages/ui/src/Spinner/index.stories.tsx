import { Spinner } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Spinner",
  component: Spinner,
};

const Template = (() => <Spinner />) as StoryFn<ComponentProps<typeof Spinner>>;

export const Main = Template.bind({});

Main.args = {};
