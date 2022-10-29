import languages from "../../i18n/languages";
import { Component } from "solid-js";

const LanguageSelect: Component<{ lang: string }> = ({ lang }) => {
  return (
    <div
      style={{
        border: "2px solid white",
        "border-radius": "0.25rem",
        "font-family": "ubuntu",
        "line-height": "1.5",
        "font-weight": "bold",
      }}
    >
      <select
        style={{
          "background-color": "#1f2937",
          color: "white",
          "font-size": "100%",
        }}
        value={lang}
        aria-label="Select language"
        onChange={(e) => {
          const newLang = e.currentTarget.value;
          const [_leadingSlash, _oldLang, ...rest] =
            window.location.pathname.split("/");
          const slug = rest.join("/");
          window.location.pathname = `/${newLang}${`${
            slug ? `/${slug}` : ""
          }`}`;
        }}
      >
        {Object.entries(languages).map(([code, name]) => (
          <option value={code}>{name}</option>
        ))}
      </select>
    </div>
  );
};

export default LanguageSelect;
