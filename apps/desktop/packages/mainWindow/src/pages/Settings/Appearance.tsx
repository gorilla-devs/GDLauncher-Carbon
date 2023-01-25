import ThemePreview from "@/components/ThemePreview";
import { queryClient, rspc } from "@/utils/rspcClient";
import { setTheme } from "@/utils/theme";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";

const Appearance = () => {
  const routeData = useRouteData();

  let mutation = rspc.createMutation(["app.setTheme"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["app.getTheme", null], newTheme);
    },
  });

  createEffect(() => {
    const themeName = () => routeData.data.data;
    setTheme(themeName());
  });

  return (
    <div class="bg-shade-8 w-full h-auto flex flex-col py-5	px-6 box-border">
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="appearance"
          options={{
            defaultValue: "Appearance",
          }}
        />
      </h2>
      <div class="flex justify-between border-box w-full max-w-[35rem]">
        <div
          class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center cursor-pointer"
          onClick={() => {
            mutation.mutate("default");
          }}
        >
          <ThemePreview
            shade1="fill-[#15181E]"
            shade2="fill-[#272B35]"
            shade3="fill-[#333947]"
          />
        </div>
        <div
          class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center cursor-pointer"
          onClick={() => {
            mutation.mutate("light");
          }}
        >
          <ThemePreview
            shade1="fill-[#380505]"
            shade2="fill-[#A90F0F]"
            shade3="fill-[#E11313]"
          />
        </div>
        <div
          class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center cursor-pointer"
          onClick={() => {
            mutation.mutate("poison-green");
          }}
        >
          <ThemePreview
            shade1="fill-[#162009]"
            shade2="fill-[#43651B]"
            shade3="fill-[#598523]"
          />
        </div>
      </div>
    </div>
  );
};

export default Appearance;
