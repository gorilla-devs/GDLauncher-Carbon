import { Button, Tabs } from "@gd/ui";
import { useNavigate, useParams } from "@solidjs/router";
import headerMockImage from "/assets/images/minecraft-forge.jpg";

const Instace = () => {
  const params = useParams();
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
            <Button
              icon={<div class="i-ri:arrow-drop-left-line" />}
              size="small"
            >
              Back
            </Button>
            <button onClick={() => navigate("/library")}>back</button>
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
