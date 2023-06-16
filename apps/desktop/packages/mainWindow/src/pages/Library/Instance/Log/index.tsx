import { streamToJson } from "@/utils/helpers";
import { Log, logsObj, setLogsObj } from "@/utils/logs";
import { port } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useParams, useRouteData } from "@solidjs/router";
import { For, Match, Switch, createEffect, createResource } from "solid-js";
import fetchData from "../instance.logs.data";

const fetchLogs = async (logId: number) => {
  return fetch(`http://localhost:${port}/instance/log?id=${logId}`);
};

const Logs = () => {
  const params = useParams();

  const routeData = useRouteData<typeof fetchData>();

  async function processStream(stream: ReadableStream<Uint8Array>) {
    if (stream.locked) {
      console.error("Stream is already locked, skipping processing");
      return;
    }

    for await (const jsonObject of streamToJson(stream)) {
      // Do something with jsonObject
      if (jsonObject) {
        console.log("jsonObject", jsonObject);
        setLogsObj(parseInt(params.id, 10), (prev) => {
          return [...(prev || []), jsonObject as Log];
        });
      }
    }
  }

  const instanceLogs = () =>
    routeData.logs.data
      ?.reverse()
      .find((item) => item.instance_id === parseInt(params.id, 10));

  const logId = () => instanceLogs()?.id;

  const [allLogs] = createResource(logId, fetchLogs);

  const instanceLogss = () => logsObj?.[parseInt(params.id, 10)] || [];

  createEffect(() => {
    console.log(
      "logsList",
      allLogs(),
      logsObj,
      parseInt(params.id, 10),
      instanceLogss()
    );
  });

  createEffect(() => {
    if (routeData.instanceDetails.data) {
      if (allLogs()?.body) {
        processStream((allLogs() as any).body);
      }
    }
  });

  return (
    <div class="overflow-y-auto pb-4 divide-y divide-darkSlate-500 max-h-full">
      <Switch>
        <Match when={(instanceLogss().length || 0) > 0}>
          <For each={instanceLogss()}>
            {(log) => {
              return (
                <div class="flex flex-col justify-center items-center w-full">
                  <pre class="m-0 w-full box-border py-2 leading-8 whitespace-pre-wrap pl-4">
                    <code class="text-md text-darkSlate-50">{log?.line}</code>
                  </pre>
                </div>
              );
            }}
          </For>
        </Match>
        <Match when={(instanceLogss().length || 0) === 0}>
          <div class="h-full flex justify-center items-center">
            <p>
              <Trans
                key="logs.no_logs"
                options={{
                  defaultValue: "No logs",
                }}
              />
            </p>
          </div>
        </Match>
      </Switch>
    </div>
  );
};

export default Logs;
