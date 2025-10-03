import { Theme } from "."

const additionalStyles = `
  html, body {
    font-size: 16px;
  }

  * {
    border-radius: 0 !important;
  }

  /* Make logo more visible with white background */
  nav img[src*="gdlauncher_wide_logo"] {
    background-color: white;
    padding: 4px 8px;
    border-radius: 6px !important;
  }
`

const frost: Theme = {
  accent: "6 182 212", // Bright cyan
  primary: "14 165 233", // Sky blue
  "darkSlate-50": "186 230 253",
  "darkSlate-100": "165 220 250",
  "darkSlate-200": "144 210 247",
  "darkSlate-300": "125 200 243",
  "darkSlate-400": "103 182 235",
  "darkSlate-500": "56 158 226",
  "darkSlate-600": "14 133 210",
  "darkSlate-700": "3 112 184",
  "darkSlate-800": "7 94 153",
  "darkSlate-900": "12 76 122",
  "lightSlate-50": "255 255 255",
  "lightSlate-100": "248 253 255",
  "lightSlate-200": "240 249 255",
  "lightSlate-300": "224 242 254",
  "lightSlate-400": "200 235 254",
  "lightSlate-500": "186 230 253",
  "lightSlate-600": "165 220 250",
  "lightSlate-700": "144 210 247",
  "lightSlate-800": "125 200 243",
  "lightSlate-900": "103 182 235",
  "darkGray-50": "203 213 225",
  "darkGray-100": "185 203 220",
  "darkGray-200": "165 190 210",
  "darkGray-300": "145 175 200",
  "darkGray-400": "125 160 190",
  "darkGray-500": "100 140 175",
  "darkGray-600": "75 120 160",
  "darkGray-700": "55 100 140",
  "darkGray-800": "40 80 120",
  "darkGray-900": "30 60 95",
  "lightGray-50": "15 25 35",
  "lightGray-100": "20 35 50",
  "lightGray-200": "30 50 70",
  "lightGray-300": "45 70 95",
  "lightGray-400": "60 90 120",
  "lightGray-500": "80 110 145",
  "lightGray-600": "105 135 170",
  "lightGray-700": "130 160 195",
  "lightGray-800": "160 185 215",
  "lightGray-900": "185 205 230",
  "primary-50": "240 249 255",
  "primary-100": "224 242 254",
  "primary-200": "186 230 253",
  "primary-300": "125 211 252",
  "primary-400": "56 189 248",
  "primary-500": "14 165 233",
  "primary-600": "2 132 199",
  "primary-700": "3 105 161",
  "primary-800": "7 89 133",
  "primary-900": "12 74 110",
  "red-50": "254 242 242",
  "red-100": "254 226 226",
  "red-200": "252 165 165",
  "red-300": "248 113 113",
  "red-400": "239 68 68",
  "red-500": "220 38 38",
  "red-600": "185 28 28",
  "red-700": "153 27 27",
  "red-800": "127 29 29",
  "red-900": "105 25 25",
  "yellow-50": "254 252 232",
  "yellow-100": "254 249 195",
  "yellow-200": "254 240 138",
  "yellow-300": "253 224 71",
  "yellow-400": "250 204 21",
  "yellow-500": "234 179 8",
  "yellow-600": "202 138 4",
  "yellow-700": "161 98 7",
  "yellow-800": "133 77 14",
  "yellow-900": "113 63 18",
  "green-50": "236 253 245",
  "green-100": "209 250 229",
  "green-200": "167 243 208",
  "green-300": "110 231 183",
  "green-400": "52 211 153",
  "green-500": "16 185 129",
  "green-600": "5 150 105",
  "green-700": "4 120 87",
  "green-800": "6 95 70",
  "green-900": "6 78 59",
  "brands-curseforge": "241 100 54",
  "brands-modrinth": "27 217 106",
  "brands-discord": "88 101 242",
  "brands-bisecthosting": "4 203 235",

  font: "Monocraft",
  "font-mono": "Monocraft",
  "ads-sidebar-background": "linear-gradient(180deg, rgba(12,76,122,1) 0%, rgba(15,25,35,1) 100%)",
  "additional-styles": additionalStyles
}

export { frost }