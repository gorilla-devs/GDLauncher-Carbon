import ThemePreview from "@/components/ThemePreview";
import { setTheme } from "@/utils/themeManager";

const Appearance = () => {
  return (
    <div class="py-5 px-6 flex justify-center">
      <div class="flex justify-between border-box w-full max-w-[35rem]">
        <div class="flex flex-col w-42 p-1 bg-[#15181E] flex justify-center items-center">
          <ThemePreview
            shade1="fill-[#15181E]"
            shade2="fill-[#272B35]"
            shade3="fill-[#333947]"
          />
          <input
            type="radio"
            name="radio-theme"
            onClick={() => {
              setTheme(0);
            }}
          />
        </div>
        <div class="flex flex-col w-42 p-1 bg-shade-9 flex justify-center items-center">
          <ThemePreview
            shade1="fill-[#380505]"
            shade2="fill-[#A90F0F]"
            shade3="fill-[#E11313]"
          />
          <input
            type="radio"
            name="radio-theme"
            onClick={() => {
              setTheme(1);
            }}
          />
        </div>
        <div class="flex flex-col w-42 p-1 bg-shade-9 flex justify-center items-center">
          <ThemePreview
            shade1="fill-shade-9"
            shade2="fill-shade-7"
            shade3="fill-shade-6"
          />
          <input type="radio" name="radio-theme" />
        </div>
      </div>
    </div>
  );
};

export default Appearance;
