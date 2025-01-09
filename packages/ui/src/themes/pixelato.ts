import { Theme } from "."

const additionalStyles = `
  html, body {
    font-size: 15px;
  }

  * {
    border-radius: 0 !important;
  }
`

const pixelato: Theme = {
  accent: "65 105 225", // Softer Royal Blue (previously Lapis Blue)
  primary: "76 175 80", // Softer Minecraft Green (previously Creeper Green)
  "darkSlate-50": "160 160 160",
  "darkSlate-100": "144 144 144",
  "darkSlate-200": "128 128 128",
  "darkSlate-300": "112 112 112",
  "darkSlate-400": "96 96 96",
  "darkSlate-500": "80 80 80",
  "darkSlate-600": "64 64 64",
  "darkSlate-700": "48 48 48",
  "darkSlate-800": "32 32 32",
  "darkSlate-900": "16 16 16",
  "lightSlate-50": "255 255 255",
  "lightSlate-100": "240 240 240",
  "lightSlate-200": "225 225 225",
  "lightSlate-300": "210 210 210",
  "lightSlate-400": "195 195 195",
  "lightSlate-500": "180 180 180",
  "lightSlate-600": "165 165 165",
  "lightSlate-700": "150 150 150",
  "lightSlate-800": "135 135 135",
  "lightSlate-900": "120 120 120",
  "darkGray-50": "112 112 112",
  "darkGray-100": "96 96 96",
  "darkGray-200": "80 80 80",
  "darkGray-300": "64 64 64",
  "darkGray-400": "48 48 48",
  "darkGray-500": "32 32 32",
  "darkGray-600": "16 16 16",
  "darkGray-700": "8 8 8",
  "darkGray-800": "4 4 4",
  "darkGray-900": "0 0 0",
  "lightGray-50": "255 255 255",
  "lightGray-100": "240 240 240",
  "lightGray-200": "225 225 225",
  "lightGray-300": "210 210 210",
  "lightGray-400": "195 195 195",
  "lightGray-500": "180 180 180",
  "lightGray-600": "165 165 165",
  "lightGray-700": "150 150 150",
  "lightGray-800": "135 135 135",
  "lightGray-900": "120 120 120",
  "primary-50": "200 230 201",
  "primary-100": "165 214 167",
  "primary-200": "129 199 132",
  "primary-300": "102 187 106",
  "primary-400": "76 175 80", // Softer Minecraft Green
  "primary-500": "67 160 71",
  "primary-600": "56 142 60",
  "primary-700": "46 125 50",
  "primary-800": "27 94 32",
  "primary-900": "0 51 0",
  "red-50": "255 205 210",
  "red-100": "255 160 160",
  "red-200": "239 154 154",
  "red-300": "229 115 115",
  "red-400": "239 83 80", // Softer Red
  "red-500": "244 67 54",
  "red-600": "229 57 53",
  "red-700": "211 47 47",
  "red-800": "198 40 40",
  "red-900": "183 28 28",
  "yellow-50": "255 253 231",
  "yellow-100": "255 249 196",
  "yellow-200": "255 245 157",
  "yellow-300": "255 241 118",
  "yellow-400": "255 238 88", // Softer Yellow
  "yellow-500": "255 235 59",
  "yellow-600": "253 216 53",
  "yellow-700": "251 192 45",
  "yellow-800": "249 168 37",
  "yellow-900": "245 127 23",
  "green-50": "225 245 254",
  "green-100": "179 229 252",
  "green-200": "129 212 250",
  "green-300": "79 195 247",
  "green-400": "65 105 225", // Softer Royal Blue
  "green-500": "3 169 244",
  "green-600": "3 155 229",
  "green-700": "2 136 209",
  "green-800": "2 119 189",
  "green-900": "1 87 155",
  "brands-curseforge": "241 100 54",
  "brands-modrinth": "27 217 106",
  "brands-discord": "88 101 242",
  "brands-bisecthosting": "4 203 235",

  font: "Monocraft",
  "font-mono": "Monocraft",
  "ads-sidebar-background": "url(./assets/images/sidebar-bg.png)",
  "additional-styles": additionalStyles
}

export { pixelato }
