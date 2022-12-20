import { Dropdown } from "./index.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";

export default {
  title: "Dropdown",
  component: Dropdown,
};

const Template = ((args) => <Dropdown {...args} />) as StoryFn<
  ComponentProps<typeof Dropdown>
>;

export const Main = Template.bind({});

Main.args = {
  options: [
    { label: "Label1", key: "key1" },
    { label: "Label2", key: "key2" },
    { label: "Label3", key: "key3" },
    { label: "Label4", key: "key4" },
  ],
  value: "key2",
  onChange: (option) => console.log("change:", option),
  error: false,
  disabled: false,
};
