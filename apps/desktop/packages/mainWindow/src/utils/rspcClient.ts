import { QueryClient } from "@tanstack/solid-query";
import {
  createClient,
  createWSClient,
  Unsubscribable,
  wsLink
} from "@rspc/client";
import { createSolidQueryHooks } from "@rspc/solid";
import type { Procedures } from "@gd/core_module";
import { createEffect } from "solid-js";

export const rspc = createSolidQueryHooks<Procedures>();
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false // default: true
    }
  }
});

export let port: number | null = null;

export default function initRspc(_port: number) {
  port = _port;
  const wsClient = createWSClient({
    url: `ws://localhost:${_port}/rspc/ws`
  });

  const client = createClient<Procedures>({
    links: [
      wsLink({
        client: wsClient
      })
    ]
  });

  const createInvalidateQuery = () => {
    const context = rspc.useContext();
    let subscription: Unsubscribable | null = null;

    function init() {
      if (!subscription) {
        subscription = client.subscription(["invalidateQuery"], {
          onData: (invalidateOperation) => {
            const key = [invalidateOperation!.key];
            if (invalidateOperation.args !== null) {
              key.concat(invalidateOperation.args);
            }
            context.queryClient.invalidateQueries(key);
          }
        });
      }
    }

    wsClient.getConnection()?.addEventListener("open", (_event) => {
      init();
    });

    wsClient.getConnection()?.addEventListener("close", (_event) => {
      subscription?.unsubscribe();
      subscription = null;
    });

    if (!subscription) {
      init();
    }
  };

  return {
    client,
    createInvalidateQuery
  };
}

export async function rspcFetch(...args) {
  const res = rspc.createQuery(...args);

  return new Promise((resolve, reject) => {
    createEffect(() => {
      if (res.error) {
        reject(res);
      } else if (res.status === "success") {
        resolve(res);
      }
    });
  });
}
