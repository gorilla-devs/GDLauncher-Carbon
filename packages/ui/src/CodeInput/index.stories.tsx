import { CodeInput } from "./index.jsx";
import type { Meta, StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

// Simple examples
export const CodeInputDefault = () => <CodeInput />;

export const CodeInputWithProps = () => <CodeInput value={"A3CBD"} />;

// example with Template
const Template = ((args) => <CodeInput {...args} />) as StoryFn<
  ComponentProps<typeof CodeInput>
>;

export const CodeInputTemplate = Template.bind({});

CodeInputTemplate.args = {
  value: "A3CBD",
};

export default {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/html/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "CodeInput",
  argTypes: {
    initialValue: { va: "string" },
  },
} as Meta<ComponentProps<typeof CodeInput>>;
