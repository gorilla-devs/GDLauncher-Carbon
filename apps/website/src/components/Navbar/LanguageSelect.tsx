import languages from "../../i18n/languages";
import {Component} from 'solid-js'

const LanguageSelect: Component<{lang: string}> = ({lang}) => {
  return (
    <div class="border-2 border-white rounded">
      <select
        class="bg-gray-800 text-white"
        value={lang}
        aria-label="Select language"
        onChange={(e) => {
          const newLang = e.target.value;
          const [_leadingSlash, _oldLang, ...rest] =
            window.location.pathname.split("/");
          const slug = rest.join("/");
          window.location.pathname = `/${newLang}/${slug}`;
        }}
      >
        {Object.entries(languages).map(([code, name]) => (
          <option value={code}>
            <span>{name}</span>
          </option>
        ))}
      </select>
    </div>
  );
}

export default LanguageSelect;
