import { Counter } from "./index.jsx";
import type { Meta, StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

// Simple examples
export const CounterDefault = () => <Counter />;

export const CounterWithProps = () => <Counter initialValue={123} />;

// example with Template
const Template = ((args) => <Counter {...args} />) as StoryFn<
  ComponentProps<typeof Counter>
>;

export const CounterTemplate = Template.bind({});

CounterTemplate.args = {
  initialValue: 10,
};

export default {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/html/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Counter",
  argTypes: {
    initialValue: { control: "number" },
  },
} as Meta<ComponentProps<typeof Counter>>;
