import ThemePreview from "@/components/ThemePreview";
import { setTheme } from "@/utils/themeManager";

const Appearance = () => {
  return (
    <div class="flex py-5 px-6">
      <div class="flex gap-4">
        <div class="w-50 p-1 bg-shade-9 flex justify-center items-center">
          <ThemePreview
            shade1="fill-shade-9"
            shade2="fill-shade-7"
            shade3="fill-shade-6"
          />
          <input
            type="radio"
            name="radio-theme"
            onClick={() => {
              setTheme(0);
            }}
          />
        </div>
        <div class="w-50 p-1 bg-shade-9 flex justify-center items-center">
          <ThemePreview />
          <input
            type="radio"
            name="radio-theme"
            onClick={() => {
              setTheme(1);
            }}
          />
        </div>
        <div class="w-50 p-1 bg-shade-9 flex justify-center items-center">
          <ThemePreview />
          <input type="radio" name="radio-theme" />
        </div>
      </div>
    </div>
  );
};

export default Appearance;
