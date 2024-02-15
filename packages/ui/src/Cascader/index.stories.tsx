import type { Meta, StoryObj } from "storybook-solidjs";
import ChildsMenu from "./ChildsMenu";
import Parent from "./Parent";

const meta: Meta<typeof Parent> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "ChildsMenu",
  component: Parent,
};

export default meta;

type Story = StoryObj<typeof Parent>;

export const WithoutParents: Story = {
  args: {
    hasSearch: false,
    isCheckbox: true,
    items: [
      {
        label: "Item 1",
        img: "",
      },
      {
        label: "Item 2",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 4",
        img: "",
      },
      {
        label: "Item 5",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 6",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
    ],
  },
};

export const WithParents: Story = {
  args: {
    hasSearch: false,
    isCheckbox: true,
    items: [
      {
        label: "Item 1",
        img: "",
      },
      {
        label: "Item 2",
        img: "",
        children: {
          items: [
            {
              label: "Item 3",
              img: "",
            },
            {
              label: "Item 3",
              img: "",
              children: {
                items: [
                  {
                    label: "Item 3",
                    img: "",
                  },
                ],
                isCheckbox: false,
                hasSearch: false,
              },
            },
            {
              label: "Item 3",
              img: "",
            },
          ],
          isCheckbox: false,
          hasSearch: false,
        },
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
      {
        label: "Item 3",
        img: "",
      },
    ],
  },
};
// export const WithTitle: Story = {
//   args: {
//     checked: true,
//     disabled: false,
//     children: "Checkbox",
//   },
// };
