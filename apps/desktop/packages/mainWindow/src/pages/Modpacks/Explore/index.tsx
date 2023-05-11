/* eslint-disable i18next/no-literal-string */
import ContentWrapper from "@/components/ContentWrapper";
import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { FEModResponse } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Tab, TabList, Tabs } from "@gd/ui";
import { Link, Outlet, useParams, useRouteData } from "@solidjs/router";
import { For } from "solid-js";
import fetchData from "../modpack.data";

const Modpack = () => {
  const navigate = useGDNavigate();
  const params = useParams();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  // <img
  //   src={(modpackDetails.data as FEModResponse).data.logo.url}
  //   class="h-full wi-full"
  //   alt={(modpackDetails.data as FEModResponse).data.logo.description}
  // />

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

  let containerRef: HTMLDivElement;
  let bgRef: HTMLDivElement;
  let innerContainerRef: HTMLDivElement;
  let refStickyContainer: HTMLDivElement;

  return (
    <ContentWrapper>
      <div
        class="relative h-full bg-darkSlate-800 overflow-auto max-h-full overflow-x-hidden"
        style={{
          "scrollbar-gutter": "stable",
        }}
        onScroll={(e) => {
          if (e.currentTarget.scrollTop > 50) {
            innerContainerRef.style.opacity = "0";
            containerRef.classList.remove("h-52");
            containerRef.classList.add("h-0");

            bgRef.classList.add("bg-darkSlate-900");

            refStickyContainer.classList.remove("h-0", "opacity-0");
            refStickyContainer.classList.add("h-20", "sticky", "top-0");
          } else {
            innerContainerRef.style.opacity = "1";
            containerRef.classList.add("h-52");
            containerRef.classList.remove("h-0");

            bgRef.classList.remove("bg-darkSlate-900");

            refStickyContainer.classList.add("h-0", "opacity-0");
            refStickyContainer.classList.remove("h-20", "sticky", "top-0");
          }
        }}
      >
        <div
          class="relative flex flex-col justify-between ease-in-out transition-all h-52 items-stretch"
          ref={(el) => {
            containerRef = el;
          }}
        >
          <div
            class="h-full absolute left-0 right-0 top-0 bg-fixed bg-cover bg-center bg-no-repeat"
            style={{
              "background-image": `url("${
                (routeData.modpackDetails?.data as FEModResponse)?.data.logo.url
              }")`,
              "background-position": "right-5rem",
            }}
            ref={(el) => {
              bgRef = el;
            }}
          />
          <div
            class="h-full"
            ref={(el) => {
              innerContainerRef = el;
            }}
          >
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
            <div
              class="flex justify-center sticky px-6 h-24 top-52 z-20"
              style={{
                background:
                  "linear-gradient(180deg, rgba(29, 32, 40, 0) 0%, #1D2028 100%)",
              }}
            >
              <div class="flex justify-center w-full">
                <div class="flex justify-between w-full max-w-185 items-end">
                  <div class="flex flex-col gap-4 w-full justify-end lg:flex-row">
                    <div class="bg-darkSlate-800 h-16 w-16 rounded-xl">
                      {/* <img /> */}
                    </div>
                    <div class="flex flex-1 flex-col max-w-185">
                      <h1 class="m-0 focus-visible:border-0 focus:outline-none focus-visible:outline-none cursor-text">
                        {params.id}
                      </h1>
                      <div class="flex flex-col lg:flex-row justify-between cursor-default">
                        <div class="flex flex-col lg:flex-row text-darkSlate-50 gap-1 items-start lg:items-center lg:gap-0">
                          <div class="p-0 lg:pr-4 border-0 lg:border-r-2 border-darkSlate-500">
                            Forge 1.19.2
                          </div>
                          <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 flex gap-2 items-center lg:px-4">
                            <div class="i-ri:time-fill" />
                            1d ago
                          </div>
                          <div class="p-0 lg:px-4 flex gap-2 items-center">
                            <div class="i-ri:user-fill" />
                            ATMTeam
                          </div>
                        </div>
                        <div class="flex items-center gap-2 mt-2 lg:mt-0">
                          <div
                            class="rounded-full flex justify-center items-center w-8 h-8"
                            style={{
                              background: "rgba(255, 255, 255, 0.1)",
                            }}
                          >
                            <div class="text-xl i-ri:more-2-fill" />
                          </div>
                          <div
                            class="rounded-full w-8 h-8 flex justify-center items-center"
                            style={{
                              background: "rgba(255, 255, 255, 0.1)",
                            }}
                          >
                            <div class="text-xl i-ri:star-s-fill" />
                          </div>
                          <Button uppercase variant="glow" size="large">
                            <Trans
                              key="instance.play"
                              options={{
                                defaultValue: "play",
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
          </div>
        </div>
        <div
          class="flex gap-4 justify-center items-center box-border w-full z-20 ease-in-out bg-darkSlate-900 px-4 h-0 opacity-0 transition-height duration-200"
          ref={(el) => {
            refStickyContainer = el;
          }}
        >
          <div class="flex items-start w-full ease-in-out transition-opacity duration-300">
            <div class="w-fit justify-center items-center transition duration-100 ease-in-out h-fit mr-4">
              <Button
                onClick={() => navigate("/library")}
                icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
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
            <div class="flex flex-1 flex-col max-w-185">
              <h4 class="m-0">{params.id}</h4>
              <div class="flex flex-col lg:flex-row justify-between">
                <div class="flex items-start lg:items-center flex-col gap-1 lg:gap-0 lg:flex-row text-darkSlate-50">
                  <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 text-xs lg:pr-2">
                    Forge 1.19.2
                  </div>
                  <div class="text-xs p-0 border-0 lg:border-r-2 border-darkSlate-500 flex gap-2 items-center lg:px-2">
                    <div class="i-ri:time-fill" />
                    1d ago
                  </div>
                  <div class="text-xs p-0 lg:px-2 flex gap-2 items-center">
                    <div class="i-ri:user-fill" />
                    ATMTeam
                  </div>
                </div>
                <div class="flex items-center gap-2 mt-2 lg:mt-0 z-10">
                  <div
                    class="rounded-full w-8 h-8 flex justify-center items-center"
                    style={{
                      background: "rgba(255, 255, 255, 0.1)",
                    }}
                  >
                    <div class="i-ri:more-2-fill text-xl" />
                  </div>
                  <div
                    class="rounded-full w-8 h-8 flex justify-center items-center"
                    style={{
                      background: "rgba(255, 255, 255, 0.1)",
                    }}
                  >
                    <div class="i-ri:star-s-fill text-xl" />
                  </div>
                  <Button uppercase variant="glow" size="small">
                    <Trans
                      key="instance.play"
                      options={{
                        defaultValue: "play",
                      }}
                    />
                  </Button>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="bg-darkSlate-800 min-h-2xl">
          <div class="flex justify-center p-6">
            <div class="bg-darkSlate-800 max-w-full w-185">
              <div class="z-20 flex flex-col bg-darkSlate-800 mb-4">
                <Tabs index={0}>
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
              <Outlet />
            </div>
          </div>
        </div>
      </div>
    </ContentWrapper>
  );
};

export default Modpack;
