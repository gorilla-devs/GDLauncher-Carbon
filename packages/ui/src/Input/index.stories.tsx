import { createEffect, createSignal } from "solid-js";
import { Input } from "./index.jsx";
import type { Meta, StoryObj } from "storybook-solidjs";

const meta: Meta<typeof Input> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Input",
  component: Input,
};

export default meta;

type Story = StoryObj<typeof Input>;

export const Main: Story = {
  args: {
    value: "Value",
    placeholder: "Type here",
    error: "",
    disabled: false,
    icon: <div class="i-ri:refresh-line" />,
  },
};

const _options = [
  {
    value: "Option 1",
    label: "Option 1",
  },
  {
    value: "Option 2",
    label: "Option 2",
  },
  {
    value: "Option 3",
    label: "Option 3",
  },
];

const [value, setValue] = createSignal("");
const [options, setOptions] = createSignal(_options);

createEffect(() => {
  console.log("OPTIONS", options());
});

export const AutoComplete: Story = {
  render: (args) => (
    <Input
      {...args}
      placeholder="Type here"
      value={value()}
      onSearch={(value: string) => {
        setValue(value);

        const newOptions = _options.filter((option) =>
          option.value.toLowerCase().includes(value.toLowerCase())
        );

        setOptions(newOptions);
      }}
      autoCompleteOptions={options()}
    />
  ),
};
