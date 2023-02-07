/* eslint-disable i18next/no-literal-string */
import getRouteIndex from "@/route/getRouteIndex";
import { Trans } from "@gd/i18n";
import { Tabs, TabList, Tab, Button } from "@gd/ui";
import { Link, Outlet, useNavigate, useParams } from "@solidjs/router";
import { For } from "solid-js";
import headerMockImage from "/assets/images/minecraft-forge.jpg";

type InstancePage = {
  label: string;
  path: string;
};

const Instance = () => {
  const navigate = useNavigate();
  const { id } = useParams();

  const instancePages = [
    {
      label: "Overview",
      path: `/library/${id}`,
    },
    {
      label: "Mods",
      path: `/library/${id}/mods`,
    },
    {
      label: "Resource Packs",
      path: `/library/${id}/resourcepacks`,
    },
    {
      label: "Screenshots",
      path: `/library/${id}/screenshots`,
    },
    {
      label: "Versions",
      path: `/library/${id}/versions`,
    },
  ];

  const selectedIndex = () =>
    getRouteIndex(instancePages, location.pathname, true);

  let ref: HTMLDivElement;

  let opacityIn = 1;
  let opacityOut = 0;

  return (
    <div
      class="relative h-full bg-shade-8 max-h-full overflow-auto overflow-x-hidden"
      style={{
        "scrollbar-gutter": "stable",
      }}
      onScroll={(e) => {
        const containerTop = e.currentTarget.getBoundingClientRect().top;
        const innerContainerTop = ref.getBoundingClientRect().top;

        const header = document.getElementById("inline-header");
        const headerContainer = document.getElementById(
          "inline-header-container"
        );

        if (innerContainerTop - containerTop >= 50) {
          if (header?.style && headerContainer?.style) {
            if (opacityOut > 0) opacityOut -= 0.1;
            headerContainer.style.height = "0";
            header.style.opacity = opacityOut.toString();
          }
        } else if (innerContainerTop - containerTop <= 10) {
          if (header?.style && headerContainer?.style) {
            if (opacityIn < 1) opacityIn += 0.1;
            headerContainer.style.height = "80px";
            header.style.opacity = opacityIn.toString();
          }
        }
      }}
    >
      <div
        class="h-52 lg:h-64 absolute top-0 left-0 right-0 bg-fixed bg-cover bg-center bg-no-repeat"
        style={{
          "background-image": `url("${headerMockImage}")`,
          "background-position": "right -5rem",
        }}
      />
      <div class="absolute top-0 left-0 right-0 h-52 lg:h-64 flex flex-col justify-between items-stretch">
        <div class="sticky top-5 left-5 w-fit">
          <Button
            onClick={() => navigate("/library")}
            icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
            size="small"
            variant="transparent"
          >
            <Trans
              key="back"
              options={{
                defaultValue: "back",
              }}
            />
          </Button>
        </div>
        <div
          class="flex justify-center px-6 h-24 absolute bottom-0 right-0 left-0"
          style={{
            background:
              "linear-gradient(180deg, rgba(29, 32, 40, 0) 0%, #1D2028 100%)",
          }}
        >
          <div class="flex justify-center w-full">
            <div class="flex justify-between items-end w-full max-w-185">
              <div class="flex flex-col lg:flex-row gap-4 justify-end w-full">
                <div class="h-16 w-16 rounded-xl bg-shade-8">
                  {/* <img /> */}
                </div>
                <div class="flex flex-1 flex-col max-w-185 ">
                  <h1 class="m-0">{id}</h1>
                  <div class="flex flex-col lg:flex-row justify-between">
                    <div class="flex items-start lg:items-center flex-col gap-1 lg:gap-0 lg:flex-row text-shade-0">
                      <div class="p-0 lg:pr-4 border-0 lg:border-r-2 border-shade-5">
                        Forge 1.19.2
                      </div>
                      <div class="p-0 lg:px-4 border-0 lg:border-r-2 border-shade-5 flex gap-2 items-center">
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
                          key="play"
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
      <div class="min-h-2xl lg:mt-64 bg-shade-8">
        <div class="mt-52 lg:mt-64 p-6 flex justify-center">
          <div class="max-w-full w-185 bg-shade-8">
            <div
              class="sticky -top-1 z-20 flex flex-col bg-shade-8 mb-4"
              ref={(el) => {
                ref = el;
              }}
            >
              <div
                class="flex flex-col lg:flex-row gap-4 justify-end w-full z-10 transition-height duration-200 ease-in-out"
                id="inline-header-container"
                style={{
                  height: 0,
                }}
              >
                <div
                  class="flex items-start w-full transition-opacity duration-300 ease-in-out mt-5"
                  id="inline-header"
                  style={{
                    opacity: 0,
                  }}
                >
                  <div class="h-fit w-fit justify-center items-center mr-4 transition duration-100 ease-in-out">
                    <Button
                      onClick={() => navigate("/library")}
                      icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
                      size="small"
                      variant="transparent"
                    >
                      <Trans
                        key="back"
                        options={{
                          defaultValue: "back",
                        }}
                      />
                    </Button>
                  </div>
                  <div class="flex flex-1 flex-col max-w-185 ">
                    <h4 class="m-0">{id}</h4>
                    <div class="flex flex-col lg:flex-row justify-between">
                      <div class="flex items-start lg:items-center flex-col gap-1 lg:gap-0 lg:flex-row text-shade-0">
                        <div class="text-xs	 p-0 lg:pr-2 border-0 lg:border-r-2 border-shade-5">
                          Forge 1.19.2
                        </div>
                        <div class="text-xs	p-0 lg:px-2 border-0 lg:border-r-2 border-shade-5 flex gap-2 items-center">
                          <div class="i-ri:time-fill" />
                          1d ago
                        </div>
                        <div class="text-xs	p-0 lg:px-2 flex gap-2 items-center">
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
                            key="play"
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
              <Tabs index={selectedIndex()}>
                <TabList>
                  <For each={instancePages}>
                    {(page: InstancePage) => (
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
  );
};

export default Instance;
