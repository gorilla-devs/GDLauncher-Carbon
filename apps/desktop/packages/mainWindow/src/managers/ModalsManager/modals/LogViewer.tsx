import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";
import {
  For,
  Show,
  createEffect,
  createResource,
  createSignal,
} from "solid-js";
import { port, rspc } from "@/utils/rspcClient";
import { streamToJson } from "@/utils/helpers";
import { Trans } from "@gd/i18n";

type Log = {
  line: string;
  type_: string;
};

const fetchLogs = async (logId: number) =>
  fetch(`http://localhost:${port}/instance/log?id=${logId}`);

const LogViewer = (props: ModalProps) => {
  const instances = rspc.createQuery(() => ["instance.getInstancesUngrouped"]);

  const logs = rspc.createQuery(() => ["instance.getLogs"]);

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} noPadding>
      <div class="h-130 w-190 overflow-hidden">
        <Show when={(logs.data?.length || 0) > 0}>
          <div class="bg-darkSlate-800 max-h-full">
            <Tabs variant="traditional">
              <div class="flex items-center max-h-full">
                <TabList>
                  <For each={logs.data}>
                    {(log) => {
                      const instance = instances.data?.find(
                        (instance) => instance.id === log.instance_id
                      );
                      return (
                        <Tab>
                          <div class="w-full flex gap-2 items-center h-10 justify-start">
                            <div class="w-6 rounded-md bg-green h-6" />
                            <p class="my-2 whitespace-nowrap">
                              {instance?.name}
                            </p>
                          </div>
                        </Tab>
                      );
                    }}
                  </For>
                </TabList>
                <div class="flex gap-4 px-5 pb-2">
                  <div class="cursor-pointer text-darkSlate-50 i-ri:upload-2-line" />
                  <div class="text-darkSlate-50 cursor-pointer i-ri:file-copy-fill" />
                </div>
              </div>
              <div class="bg-darkSlate-700 overflow-y-auto max-h-130">
                <For each={logs.data}>
                  {(instance) => {
                    const [logsArray, setLogsArray] = createSignal<Log[]>([]);

                    const [logs] = createResource(instance.id, fetchLogs);

                    async function processStream(
                      stream: ReadableStream<Uint8Array>
                    ) {
                      for await (const jsonObject of streamToJson(stream)) {
                        // Do something with jsonObject
                        setLogsArray((prev) => [...prev, jsonObject as Log]);
                      }
                    }

                    createEffect(() => {
                      if (logs()?.body) {
                        processStream((logs() as any).body);
                      }
                    });

                    return (
                      <TabPanel>
                        <div class="overflow-y-auto divide-y divide-darkSlate-500 pb-4">
                          <For each={logsArray()}>
                            {(log) => {
                              return (
                                <div class="flex flex-col justify-center items-center w-full">
                                  <pre class="m-0 py-2 w-full box-border leading-8 whitespace-pre-wrap pl-4">
                                    <code>{log?.line}</code>
                                  </pre>
                                </div>
                              );
                            }}
                          </For>
                        </div>
                      </TabPanel>
                    );
                  }}
                </For>
              </div>
            </Tabs>
          </div>
        </Show>
        <Show when={(logs.data?.length || 0) === 0}>
          <div class="bg-darkSlate-700 h-full flex justify-center items-center">
            <p>
              <Trans
                key="logs.no_logs"
                options={{
                  defaultValue: "No logs",
                }}
              />
            </p>
          </div>
        </Show>
      </div>
    </ModalLayout>
  );
};

export default LogViewer;
