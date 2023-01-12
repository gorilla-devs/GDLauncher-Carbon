import { Tabs } from "./Tabs.jsx";
import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";
import Tab from "./Tab.jsx";
import TabList from "./TabList.jsx";
import TabPanel from "./TabPanel.jsx";

export default {
  title: "Tabs",
  component: Tabs,
  argTypes: {
    type: {
      options: ["underline", "block"],
      control: { type: "radio" },
    },
    orientation: {
      options: ["horizontal", "veritcal"],
      control: { type: "radio" },
    },
  },
};

const Template = ((args) => (
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
)) as StoryFn<ComponentProps<typeof Tabs>>;

export const Main = Template.bind({});

Main.args = {};
