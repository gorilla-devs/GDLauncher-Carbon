/* eslint-disable i18next/no-literal-string */
import ContentWrapper from "@/components/ContentWrapper";
import { useGDNavigate } from "@/managers/NavigationManager";
import { FEModResponse } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Tab, TabList, Tabs } from "@gd/ui";
import { Link, Outlet, useParams, useRouteData } from "@solidjs/router";
import { For, Show } from "solid-js";
import fetchData from "../modpack.overview";
import { mappedMcVersions } from "@/utils/mcVersion";
import { format } from "date-fns";

const Modpack = () => {
  const navigate = useGDNavigate();
  const params = useParams();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const instancePages = () => [
    {
      label: "Overview",
      path: `/modpacks/${params.id}`,
    },
    {
      label: "Versions",
      path: `/modpacks/${params.id}/versions`,
    },
  ];

  // eslint-disable-next-line no-unused-vars
  // let containerRef: HTMLDivElement;
  // let bgRef: HTMLDivElement;
  // let innerContainerRef: HTMLDivElement;
  // let refStickyContainer: HTMLDivElement;

  return (
    <ContentWrapper>
      <div
        class="relative h-full bg-darkSlate-800 overflow-auto max-h-full overflow-x-hidden"
        style={{
          "scrollbar-gutter": "stable",
        }}
      >
        <div
          class="flex flex-col justify-between ease-in-out transition-all h-52 items-stretch"
          // ref={(el) => {
          //   containerRef = el;
          // }}
        >
          <div
            class="relative h-full"
            // ref={(el) => {
            //   innerContainerRef = el;
            // }}
          >
            <div
              class="h-full absolute left-0 right-0 top-0 bg-fixed bg-cover bg-center bg-no-repeat"
              style={{
                "background-image": `url("${
                  (routeData.modpackDetails?.data as FEModResponse)?.data.logo
                    .url
                }")`,
                "background-position": "right-5rem",
              }}
              // ref={(el) => {
              //   bgRef = el;
              // }}
            />
            <div class="z-10 top-5 sticky left-5 w-fit">
              <Button
                onClick={() => navigate("/modpacks")}
                icon={<div class="text-2xl i-ri:arrow-drop-left-line" />}
                size="small"
                variant="transparent"
              >
                <Trans
                  key="instance.step_back"
                  options={{
                    defaultValue: "Back",
                  }}
                />
              </Button>
            </div>
            <div class="flex justify-center sticky px-4 h-24 top-52 z-20 bg-gradient-to-t from-darkSlate-800 from-10%">
              <div class="flex gap-4 w-full lg:flex-row">
                <div
                  class="bg-darkSlate-800 h-16 w-16 rounded-xl bg-center bg-cover"
                  style={{
                    "background-image": `url("${
                      (routeData.modpackDetails?.data as FEModResponse)?.data
                        .logo.thumbnailUrl
                    }")`,
                  }}
                />
                <div class="flex flex-1 flex-col max-w-185">
                  <div
                    class="flex gap-4 items-center cursor-pointer"
                    onClick={() => {
                      window.openExternalLink(
                        routeData.modpackDetails?.data?.data.links.websiteUrl ||
                          ""
                      );
                    }}
                  >
                    <h1 class="m-0">
                      {
                        (routeData.modpackDetails?.data as FEModResponse)?.data
                          .name
                      }
                    </h1>
                    <div class="i-ri:external-link-line text-2xl" />
                  </div>
                  <div class="flex flex-col lg:flex-row justify-between cursor-default">
                    <div class="flex flex-col lg:flex-row text-darkSlate-50 gap-1 items-start lg:items-center lg:gap-0">
                      <div class="p-0 lg:pr-4 border-0 lg:border-r-2 border-darkSlate-500">
                        {
                          routeData.modpackDetails.data?.data
                            .latestFilesIndexes[0].gameVersion
                        }
                      </div>
                      <Show
                        when={routeData.modpackDetails.data?.data.dateCreated}
                      >
                        <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 flex gap-2 items-center lg:px-4">
                          <div class="i-ri:time-fill" />
                          {format(
                            new Date(
                              (
                                routeData.modpackDetails.data as FEModResponse
                              ).data.dateCreated
                            ).getTime(),
                            "P"
                          )}
                        </div>
                      </Show>
                      <div class="p-0 lg:px-4 flex gap-2 items-center">
                        <div class="i-ri:user-fill" />
                        <div class="text-sm whitespace-nowrap flex gap-2 max-w-52 overflow-x-auto">
                          <For
                            each={routeData.modpackDetails.data?.data.authors}
                          >
                            {(author) => <p class="m-0">{author.name}</p>}
                          </For>
                        </div>
                      </div>
                    </div>
                    <div class="flex items-center gap-2 mt-2 lg:mt-0">
                      <Dropdown
                        options={mappedMcVersions()}
                        icon={<div class="i-ri:price-tag-3-fill" />}
                        rounded
                        bgColorClass="bg-darkSlate-400"
                        value={mappedMcVersions()[0].key}
                        onChange={() => {}}
                      />
                      <Button uppercase variant="glow" size="large">
                        <Trans
                          key="modpack.download"
                          options={{
                            defaultValue: "Download",
                          }}
                        />
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="bg-darkSlate-800">
          <div class="flex justify-center px-4 pb-4">
            <div class="bg-darkSlate-800 w-full">
              <div class="sticky top-0 flex flex-col mb-4 z-0">
                <Tabs>
                  <TabList>
                    <For each={instancePages()}>
                      {(page) => (
                        <Link href={page.path} class="no-underline">
                          <Tab class="bg-transparent">{page.label}</Tab>
                        </Link>
                      )}
                    </For>
                  </TabList>
                </Tabs>
              </div>
              <div>
                <Outlet />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ContentWrapper>
  );
};

export default Modpack;
