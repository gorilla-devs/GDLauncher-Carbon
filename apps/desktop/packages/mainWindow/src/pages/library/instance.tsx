import { Button, Tabs } from "@gd/ui";
import { useNavigate, useParams } from "@solidjs/router";
import Overview from "./tabs/Overview";
import headerMockImage from "/assets/images/minecraft-forge.jpg";

const Instace = () => {
  const navigate = useNavigate();
  const params = useParams();

  const tabs = [
    {
      name: "Overview",
      component: <Overview />,
    },
    {
      name: "Mods",
      component: <div>Mods</div>,
    },
    {
      name: "Resource packs",
      component: <div>Resource packs</div>,
    },
    {
      name: "Screenshots",
      component: <div>Screenshots</div>,
    },
    {
      name: "Versions",
      component: <div>Versions</div>,
    },
    {
      name: "Changelog",
      component: <div>Changelog</div>,
    },
    {
      name: "Logs",
      component: <div>Logs</div>,
    },
  ];

  return (
    <>
      <div
        class="relative h-full bg-black-black max-h-full overflow-auto overflow-x-hidden"
        style={{
          "scrollbar-gutter": "stable",
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
          <Button
            class="mt-5 ml-5"
            onClick={() => navigate("/library")}
            icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
            size="small"
            type="transparent"
          >
            Back
          </Button>
          <div
            class="flex justify-center px-6 h-24"
            style={{
              background:
                "linear-gradient(180deg, rgba(29, 32, 40, 0) 0%, #1D2028 100%)",
            }}
          >
            <div class="flex justify-center w-full">
              <div class="flex justify-between items-end w-full max-w-185">
                <div class="flex flex-col lg:flex-row gap-4 justify-end w-full">
                  <div class="h-16 w-16 rounded-xl bg-black-black">
                    {/* <img /> */}
                  </div>
                  <div class="flex flex-1 flex-col max-w-185 ">
                    <h1 class="m-0">{params.id}</h1>
                    <div class="flex flex-col lg:flex-row justify-between">
                      <div class="flex items-start lg:items-center flex-col gap-1 lg:gap-0 lg:flex-row text-shade-0">
                        <div class="p-0 lg:pr-4 border-0 lg:border-r-2 border-black-gray">
                          Forge 1.19.2
                        </div>
                        <div class="p-0 lg:px-4 border-0 lg:border-r-2 border-black-gray flex gap-2 items-center">
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
                        <Button uppercase type="glow" size="small">
                          Play
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="mt-52 lg:mt-64 bg-black-black">
          <div class="mt-52 lg:mt-64 p-6 flex justify-center">
            <div class="max-w-full w-185">
              <Tabs tabs={tabs} />
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

export default Instace;
