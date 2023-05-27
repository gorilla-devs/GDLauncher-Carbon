import type { StorybookConfig } from "storybook-solidjs-vite";
import Unocss from "unocss/vite";
import { unocssConfig } from "@gd/config";

const config: StorybookConfig = {
  stories: ["../src/**/*.mdx", "../src/**/*.stories.@(js|jsx|ts|tsx)"],
  addons: [
    "@storybook/addon-links",
    "@storybook/addon-essentials",
    "@storybook/addon-interactions",
  ],
  framework: {
    name: "storybook-solidjs-vite",
    options: {},
  },
  docs: {
    autodocs: "tag",
  },
  viteFinal(config) {
    config.plugins?.push(Unocss(unocssConfig as any));
    // Add other configuration here depending on your use case
    return config;
  },
};
export default config;
