import type { StoryFn } from "@storybook/html";
import type { ComponentProps } from "solid-js";
import { mainTheme, Theme } from "../themes";
import { Tabs } from "./Tabs.jsx";
import { Tab } from "./Tab.jsx";
import { TabList } from "./TabList.jsx";
import { TabPanel } from "./TabPanel.jsx";

export default {
  title: "Tabs",
  component: Tabs,
  argTypes: {
    type: {
      options: ["underline", "block"],
      control: { type: "radio" },
    },
    orientation: {
      options: ["horizontal", "vertical"],
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

applyTheme(mainTheme);

export const Main = Template.bind({});

Main.args = {
  index: 0,
};

function applyTheme(theme: Theme) {
  // Inject theme
  for (const key in theme) {
    document.documentElement.style.setProperty(
      `--${key}`,
      theme[key as keyof Theme]
    );
  }
}
