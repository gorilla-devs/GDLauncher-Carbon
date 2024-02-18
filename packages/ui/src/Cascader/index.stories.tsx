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
    isParent: false,
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
    isCheckbox: false,
    isParent: true,
    items: [
      {
        label: "Item 1",
        img: "",
      },
      {
        label: "Item 2",
        img: "",
        children: {
          isParent: false,
          items: [
            {
              label: "Subitem 2.1",
              img: "",
            },
            {
              label: "Subitem 2.2",
              img: "",
              children: {
                isParent: false,
                items: [
                  {
                    label: "Subsubitem 2.2.1",
                    img: "",
                  },
                ],
                isCheckbox: false,
                hasSearch: false,
              },
            },
            {
              label: "Subitem 2.3",
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
        children: {
          isParent: false,
          items: [
            {
              label: "Subitem 3.1",
              img: "",
            },
          ],
          isCheckbox: false,
          hasSearch: false,
        },
      },
      {
        label: "Item 4",
        img: "",
        children: {
          isParent: false,
          items: [
            {
              label: "Subitem 4.1",
              img: "",
            },
          ],
          isCheckbox: false,
          hasSearch: false,
        },
      },
      {
        label: "Item 5",
        img: "",
      },
      {
        label: "Item 6",
        img: "",
      },
      {
        label: "Item 7",
        img: "",
      },
      {
        label: "Item 8",
        img: "",
      },
      {
        label: "Item 9",
        img: "",
      },
      {
        label: "Item 10",
        img: "",
      },
      {
        label: "Item 11",
        img: "",
      },
      {
        label: "Item 12",
        img: "",
      },
      {
        label: "Item 13",
        img: "",
      },
    ],
  },
};
