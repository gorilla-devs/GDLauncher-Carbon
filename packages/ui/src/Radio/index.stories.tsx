import { createSignal } from "solid-js";
import { Radio } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const [value, setValue] = createSignal("1");

const meta: Meta<typeof Radio.group> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Radio.group",
  component: Radio.group,
  args: {
    onChange: setValue,
    options: [
      {
        value: "1",
        label: "Option 1",
      },
      {
        value: "2",
        label: "Option 2",
      },
      {
        value: "3",
        label: "Option 3",
      },
    ],
  },
};

export default meta;

type Story = StoryObj<typeof Radio.group>;

export const Main: Story = {
  render: (args) => <Radio.group value={value()} {...args} />,
};

export const ButtonStyle: Story = {
  render: (args) => (
    <Radio.group buttonStyle="button" value={value()} {...args} />
  ),
};
