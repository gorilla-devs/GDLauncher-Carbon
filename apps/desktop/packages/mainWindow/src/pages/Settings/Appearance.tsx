import { setTheme } from "@/utils/themeManager";

const Appearance = () => {
  return (
    <div class="flex">
      <button
        onClick={() => {
          setTheme(0);
        }}
      >
        Tema1
      </button>
      <button
        onClick={() => {
          setTheme(1);
        }}
      >
        Tema2
      </button>
    </div>
  );
};

export default Appearance;
