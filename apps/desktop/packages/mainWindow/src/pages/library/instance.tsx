import { Button, Tabs } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import Overview from "./tabs/Overview";
import headerMockImage from "/assets/images/minecraft-forge.jpg";

const Instace = () => {
  const navigate = useNavigate();

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
      <div class="relative h-full bg-black-black max-h-full overflow-auto">
        <div
          class="h-64 absolute top-0 left-0 right-0 bg-fixed bg-cover bg-center bg-no-repeat"
          style={{
            "background-image": `url("${headerMockImage}")`,
            "background-position": "right -5rem",
          }}
        />
        <div class="mt-64 h-200 bg-black-black">
          <div class="h-65 absolute top-0 left-0">
            <div class="absolute top-5 left-5">
              <Button
                onClick={() => navigate("/library")}
                icon={<div class="i-ri:arrow-drop-left-line text-2xl" />}
                size="small"
              >
                Back
              </Button>
            </div>
          </div>
          <div class="mt-65 px-6">
            <Tabs tabs={tabs} />
          </div>
        </div>
      </div>
    </>
  );
};

export default Instace;
