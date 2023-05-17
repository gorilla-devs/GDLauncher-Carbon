import type { Meta, StoryObj } from "storybook-solidjs";
import { Tabs } from "./Tabs.jsx";
import { Tab } from "./Tab.jsx";
import { TabList } from "./TabList.jsx";
import { TabPanel } from "./TabPanel.jsx";

const meta: Meta<typeof Tabs> = {
  /* 👇 The title prop is optional.
   * See https://storybook.js.org/docs/solid/configure/overview#configure-story-loading
   * to learn how to generate automatic titles
   */
  title: "Tabs",
  component: Tabs,
  argTypes: {
    // type: {
    //   // options: ["underline", "block"],
    //   // control: { type: "radio" },
    // },
    orientation: {
      options: ["horizontal", "vertical"],
      control: { type: "radio" },
    },
  },
};

export default meta;

type Story = StoryObj<typeof Tabs>;

export const Main: Story = {
  render: (args) => (
    <Tabs {...args}>
      <TabList>
        <Tab>One</Tab>
        <Tab>Two</Tab>
        <Tab>Three</Tab>
      </TabList>
      <TabPanel>1</TabPanel>
      <TabPanel>2</TabPanel>
      <TabPanel>3</TabPanel>
    </Tabs>
  ),
};
