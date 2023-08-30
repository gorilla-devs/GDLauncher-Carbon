/* eslint-disable solid/no-innerhtml */
import { useRouteData } from "@solidjs/router";
import { Match, Suspense, Switch } from "solid-js";
import { Skeleton } from "@gd/ui";
import { marked } from "marked";
import sanitizeHtml from "sanitize-html";
import { MRFEProject } from "@gd/core_module/bindings";
import fetchData from "../modpack.overview";

const Overview = () => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const Description = () => {
    const cleanHtml = () =>
      sanitizeHtml(routeData.modpackDescription?.data?.data || "", {
        allowedTags: sanitizeHtml.defaults.allowedTags.concat([
          "img",
          "iframe",
        ]),
        allowedAttributes: {
          a: ["href", "name", "target", "class"],
          img: ["src", "width", "height"],
          iframe: ["src", "width", "height", "allowfullscreen"],
        },
        allowedIframeHostnames: [
          "www.youtube.com",
          "i.imgur.com",
          "cdn.ko-fi.com",
        ],
        transformTags: {
          a: sanitizeHtml.simpleTransform("a", { class: "text-blue-500" }),
        },
      });

    return (
      <Suspense fallback={<Skeleton.modpackOverviewPage />}>
        <div>
          <Switch fallback={<Skeleton.modpackOverviewPage />}>
            <Match when={routeData.isCurseforge}>
              <div innerHTML={cleanHtml()} />
            </Match>
            <Match when={!routeData.isCurseforge}>
              <div
                class="w-full"
                innerHTML={marked.parse(
                  sanitizeHtml(
                    (routeData.modpackDetails.data as MRFEProject)?.body || ""
                  )
                )}
              />
            </Match>
          </Switch>
        </div>
      </Suspense>
    );
  };

  return (
    <Switch fallback={<Skeleton.modpackOverviewPage />}>
      <Match when={!routeData.modpackDescription?.isLoading}>
        <Description />
      </Match>
      <Match when={routeData.modpackDescription?.isLoading}>
        <Skeleton.modpackOverviewPage />
      </Match>
    </Switch>
  );
};

export default Overview;
