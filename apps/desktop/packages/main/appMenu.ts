import { Menu, app } from "electron";

const edit = [
  ...(process.platform === "darwin"
    ? [
        {
          label: "GDLauncher",
          submenu: [
            {
              label: "About GDLauncher",
              role: "about",
            },
            { type: "separator" },
            {
              label: "Services",
              role: "services",
              submenu: [],
            },
            { type: "separator" },
            {
              label: "Hide GDLauncher",
              accelerator: "Command+H",
              role: "hide",
            },
            {
              label: "Hide Others",
              accelerator: "Command+Alt+H",
              role: "hideOthers",
            },
            {
              label: "Show All",
              role: "unhide",
            },
            { type: "separator" },
            {
              label: "Quit GDLauncher",
              accelerator: "Command+Q",
              click: () => {
                app.quit();
              },
            },
          ],
        },
      ]
    : []),
  {
    label: "Edit",
    submenu: [
      {
        label: "Cut",
        accelerator: "CmdOrCtrl+X",
        selector: "cut:",
      },
      {
        label: "Copy",
        accelerator: "CmdOrCtrl+C",
        selector: "copy:",
      },
      {
        label: "Paste",
        accelerator: "CmdOrCtrl+V",
        selector: "paste:",
      },
      {
        label: "Select All",
        accelerator: "CmdOrCtrl+A",
        selector: "selectAll:",
      },
      { type: "separator" },
      { label: "Undo", accelerator: "CmdOrCtrl+Z", selector: "undo:" },
      { label: "Redo", accelerator: "Shift+CmdOrCtrl+Z", selector: "redo:" },
    ],
  },
];

Menu.setApplicationMenu(Menu.buildFromTemplate(edit as any));
