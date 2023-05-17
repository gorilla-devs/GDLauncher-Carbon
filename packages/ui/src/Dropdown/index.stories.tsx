import { Dropdown } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Dropdown> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Dropdown",
  component: Dropdown,
};

export default meta;

type Story = StoryObj<typeof Dropdown>;

export const Main: Story = {
  args: {
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
    rounded: true,
    label: "Name",
  },
};
