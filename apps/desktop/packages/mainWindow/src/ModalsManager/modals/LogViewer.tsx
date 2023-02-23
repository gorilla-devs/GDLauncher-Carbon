/* eslint-disable i18next/no-literal-string */
import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { For, Show } from "solid-js";

const logs = [
  {
    type: "",
    data: ` 
        <log4j:Event logger="net.minecraft.client.renderer.texture.TextureAtlas" timestamp="1665663352995" level="INFO" thread="Render thread">
            <log4j:Message>
                <![CDATA[Created: 256x256x0 minecraft:textures/atlas/paintings.png-atlas]]></log4j:Message>
            <log4j:Message>
            <![CDATA[Created: 128x128x0 minecraft:textures/atlas/mob_effects.png-atlas]]></log4j:Message>
        </log4j:Event>
    `,
  },
  {
    type: "",
    data: `
          <log4j:Event logger="net.minecraft.client.renderer.texture.TextureAtlas" timestamp="1665663352995" level="INFO" thread="Render thread">
              <log4j:Message>
                  <![CDATA[Created: 256x256x0 minecraft:textures/atlas/paintings.png-atlas]]></log4j:Message>
              <log4j:Message>
              <![CDATA[Created: 128x128x0 minecraft:textures/atlas/mob_effects.png-atlas]]></log4j:Message>
          </log4j:Event>
      `,
  },
  {
    type: "",
    data: `
          <log4j:Event logger="net.minecraft.client.renderer.texture.TextureAtlas" timestamp="1665663352995" level="INFO" thread="Render thread">
              <log4j:Message>
                  <![CDATA[Created: 256x256x0 minecraft:textures/atlas/paintings.png-atlas]]></log4j:Message>
              <log4j:Message>
              <![CDATA[Created: 128x128x0 minecraft:textures/atlas/mob_effects.png-atlas]]></log4j:Message>
          </log4j:Event>
      `,
  },
];

const LogViewer = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="h-130 w-190 overflow-hidden">
        <div class="bg-shade-8 max-h-full">
          <Tabs variant="traditional">
            <div class="flex items-center max-h-full">
              <TabList>
                <Tab>
                  <div class="w-full h-10 flex gap-2 items-center justify-start">
                    <div class="w-6 h-6 rounded-md bg-green" />
                    <p class="my-2">Valhelsia</p>
                  </div>
                </Tab>
                <Tab>
                  <div class="w-full h-10 flex gap-2 items-center justify-start">
                    <div class="w-6 h-6 rounded-md bg-green" />
                    <p class="my-2">Valhelsia</p>
                  </div>
                </Tab>
                <Tab>
                  <div class="w-full h-10 flex gap-2 items-center justify-start">
                    <div class="w-6 h-6 rounded-md bg-green" />
                    <p class="my-2">Valhelsia</p>
                  </div>
                </Tab>
                <Tab>
                  <div class="w-full h-10 flex gap-2 items-center justify-start">
                    <div class="w-6 h-6 rounded-md bg-green" />
                    <p class="my-2">Valhelsia</p>
                  </div>
                </Tab>
              </TabList>
              <div class="flex gap-4 px-5">
                <div class="i-ri:upload-2-line text-shade-0 cursor-pointer" />
                <div class="i-ri:file-copy-fill text-shade-0 cursor-pointer" />
              </div>
            </div>
            <div class="bg-shade-7 overflow-y-auto max-h-130">
              <TabPanel>
                <div class="divide-y divide-shade-5 overflow-y-auto">
                  <For each={logs}>
                    {(log) => (
                      <div class="flex flex-col justify-center items-center">
                        <pre class="leading-8 whitespace-pre-wrap m-0 pl-4">
                          {log.data}
                        </pre>
                      </div>
                    )}
                  </For>
                </div>
              </TabPanel>
              <TabPanel>2</TabPanel>
              <TabPanel>3</TabPanel>
              <TabPanel>4</TabPanel>
            </div>
          </Tabs>
        </div>
      </div>
    </ModalLayout>
  );
};

export default LogViewer;
