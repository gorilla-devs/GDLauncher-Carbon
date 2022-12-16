import { Button, Tabs } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import headerMockImage from "/assets/images/minecraft-forge.jpg";

const Instace = () => {
  const navigate = useNavigate();

  return (
    <>
      <div
        class="relative h-full bg-fixed bg-no-repeat max-h-full overflow-auto"
        style={{
          "background-image": `url("${headerMockImage}")`,
          "background-position": "center -5rem",
        }}
      >
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
            <Tabs
              tabs={[
                {
                  name: "mods",
                  component: <div>mods jsx</div>,
                  icon: "image-2-fill",
                },
                {
                  name: "modpacks",
                  component: <div>modpacks jsx</div>,
                },
              ]}
            />
          </div>
        </div>
      </div>
    </>
  );
};

export default Instace;
