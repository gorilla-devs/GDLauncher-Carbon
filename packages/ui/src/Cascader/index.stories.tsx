import type { Meta, StoryObj } from "storybook-solidjs";
import ChildsMenu from "./ChildsMenu";

const meta: Meta<typeof ChildsMenu> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "ChildsMenu",
  component: ChildsMenu,
};

export default meta;

type Story = StoryObj<typeof ChildsMenu>;

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
        children: {
          items: [
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
