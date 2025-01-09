import { Theme } from "."

const additionalStyles = `
  html, body {
    font-size: 16px;
    color: rgb(var(--lightSlate-50)) !important;
  }

  * {
    border-radius: 0 !important;
  }

  button, div:has(> input:not([type="checkbox"]):not([type="radio"])) {
    border: 2px solid !important;
    border-color: #dfdfdf #000 #000 #dfdfdf !important;
  }

  button {
      color: rgb(var(--darkSlate-50)) !important;
  }

  button:hover {
    color: rgb(var(--darkSlate-50)) !important;
  }

  button:active {
    border-color: #000 #dfdfdf #dfdfdf #000 !important;
    box-shadow: -1px -1px 0 #dfdfdf inset, 1px 1px 0 #808080 inset !important;
  }
`

const win95: Theme = {
  accent: "0 0 128", // Navy Blue
  primary: "0 0 128", // Windows 95 Blue
  "darkSlate-50": "255 255 255", // White
  "darkSlate-100": "245 245 245",
  "darkSlate-200": "235 235 235",
  "darkSlate-300": "225 225 225", // Light gray (closer to Win95 background)
  "darkSlate-400": "215 215 215",
  "darkSlate-500": "205 205 205",
  "darkSlate-600": "195 195 195",
  "darkSlate-700": "185 185 185",
  "darkSlate-800": "175 175 175",
  "darkSlate-900": "165 165 165",
  "lightSlate-50": "80 80 80", // Darkest gray for text
  "lightSlate-100": "70 70 70",
  "lightSlate-200": "60 60 60",
  "lightSlate-300": "50 50 50",
  "lightSlate-400": "40 40 40",
  "lightSlate-500": "30 30 30",
  "lightSlate-600": "20 20 20",
  "lightSlate-700": "10 10 10",
  "lightSlate-800": "5 5 5",
  "lightSlate-900": "0 0 0", // Black
  "darkGray-50": "192 192 192", // Typical Win95 button face color
  "darkGray-100": "176 176 176",
  "darkGray-200": "160 160 160",
  "darkGray-300": "144 144 144",
  "darkGray-400": "128 128 128", // Win95 gray text
  "darkGray-500": "112 112 112",
  "darkGray-600": "96 96 96",
  "darkGray-700": "80 80 80",
  "darkGray-800": "64 64 64",
  "darkGray-900": "48 48 48",
  "lightGray-50": "255 255 255", // Kept as is
  "lightGray-100": "240 240 240",
  "lightGray-200": "225 225 225",
  "lightGray-300": "210 210 210",
  "lightGray-400": "195 195 195",
  "lightGray-500": "180 180 180",
  "lightGray-600": "165 165 165",
  "lightGray-700": "150 150 150",
  "lightGray-800": "135 135 135",
  "lightGray-900": "120 120 120",
  "primary-50": "200 200 255", // Lightest blue
  "primary-100": "180 180 255",
  "primary-200": "160 160 255",
  "primary-300": "140 140 255",
  "primary-400": "120 120 255",
  "primary-500": "0 0 255", // Bright blue
  "primary-600": "0 0 225",
  "primary-700": "0 0 195",
  "primary-800": "0 0 165",
  "primary-900": "0 0 128", // Windows 95 Blue
  "red-50": "255 192 192",
  "red-100": "255 128 128",
  "red-200": "255 64 64",
  "red-300": "255 0 0",
  "red-400": "192 0 0",
  "red-500": "128 0 0", // Windows 95 Dark Red
  "red-600": "64 0 0",
  "red-700": "32 0 0",
  "red-800": "16 0 0",
  "red-900": "64 0 0", // Very dark red instead of black
  "yellow-50": "255 255 192",
  "yellow-100": "255 255 128",
  "yellow-200": "255 255 64",
  "yellow-300": "255 255 0",
  "yellow-400": "192 192 0",
  "yellow-500": "128 128 0", // Windows 95 Olive
  "yellow-600": "64 64 0",
  "yellow-700": "32 32 0",
  "yellow-800": "16 16 0",
  "yellow-900": "32 32 0", // Very dark yellow (olive) instead of black
  "green-50": "192 255 192",
  "green-100": "128 255 128",
  "green-200": "64 255 64",
  "green-300": "0 255 0",
  "green-400": "0 192 0",
  "green-500": "0 128 0", // Windows 95 Green
  "green-600": "0 64 0",
  "green-700": "0 32 0",
  "green-800": "0 16 0",
  "green-900": "0 32 0", // Very dark green instead of black
  "brands-curseforge": "241 100 54",
  "brands-modrinth": "27 217 106",
  "brands-discord": "88 101 242",
  "brands-bisecthosting": "4 203 235",

  font: "W95FA",
  "font-mono": "W95FA",
  "ads-sidebar-background": "rgb(var(--accent))",
  "additional-styles": additionalStyles
}

export { win95 }
