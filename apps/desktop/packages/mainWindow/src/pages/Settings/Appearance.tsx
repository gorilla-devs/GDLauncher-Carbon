import ThemePreview from "@/components/ThemePreview";
import { setTheme } from "@/utils/theme";

const Appearance = () => {
  return (
    <div class="py-5 px-6 flex justify-center">
      <div class="flex justify-between border-box w-full max-w-[35rem]">
        <div
          class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center cursor-pointer"
          onClick={() => {
            setTheme(0);
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
            setTheme(1);
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
            setTheme(2);
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
