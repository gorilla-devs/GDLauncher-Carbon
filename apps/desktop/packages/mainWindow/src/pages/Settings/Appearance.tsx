/* eslint-disable i18next/no-literal-string */
import { setTheme } from "@/utils/theme";

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
