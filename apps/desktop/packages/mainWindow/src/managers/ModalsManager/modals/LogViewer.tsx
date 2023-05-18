/* eslint-disable i18next/no-literal-string */
import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import { For, createEffect } from "solid-js";
import { port, rspc } from "@/utils/rspcClient";
import { getRunningState } from "@/utils/instances";
import { UngroupedInstance } from "@gd/core_module/bindings";

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

async function streamToJson(stream) {
  // Step 1: Read all data from the stream
  const reader = stream.getReader();
  const chunks = [];
  let done, value;

  while (!done) {
    ({ done, value } = await reader.read());
    if (value) {
      chunks.push(value);
    }
  }

  // Step 2: Convert the Uint8Array to a string
  const decoder = new TextDecoder();
  const string = chunks.map((chunk) => decoder.decode(chunk)).join("");

  // Step 3: Parse the string into JSON
  const json = JSON.parse(string);

  return json;
}

const LogViewer = (props: ModalProps) => {
  const instances = rspc.createQuery(() => ["instance.getInstancesUngrouped"]);

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="h-130 w-190 overflow-hidden">
        <div class="bg-darkSlate-800 max-h-full">
          <Tabs variant="traditional">
            <div class="flex items-center max-h-full">
              <TabList>
                <For each={instances.data}>
                  {(instance) => {
                    const runningState = getRunningState(instance.status);
                    if (runningState) {
                      fetch(
                        `http://localhost:${port}/instance/log?id=${runningState?.log_id}`
                      ).then(async (log) => {
                        console.log("LOG", log, log.body);
                        streamToJson(log.body).then((AA) => {
                          console.log("AAA", AA);
                        });
                      });
                    }

                    return (
                      <Tab>
                        <div class="w-full flex gap-2 items-center h-10 justify-start">
                          <div class="w-6 rounded-md bg-green h-6" />
                          <p class="my-2">{instance.name}</p>
                        </div>
                      </Tab>
                    );
                  }}
                </For>
                {/* <Tab>
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
                </Tab> */}
              </TabList>
              <div class="flex gap-4 px-5">
                <div class="cursor-pointer text-darkSlate-50 i-ri:upload-2-line" />
                <div class="text-darkSlate-50 cursor-pointer i-ri:file-copy-fill" />
              </div>
            </div>
            <div class="bg-darkSlate-700 overflow-y-auto max-h-130">
              <TabPanel>
                <div class="overflow-y-auto divide-y divide-darkSlate-500">
                  <For each={logs}>
                    {(log) => (
                      <div class="flex flex-col justify-center items-center">
                        <pre class="m-0 leading-8 whitespace-pre-wrap pl-4">
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
