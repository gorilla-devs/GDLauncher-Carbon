module.exports = {
  normalizeFilenames: /^.+?(\.\w+?)?\..+$/,
  files: [
    {
      path: "dist/main/**/*.*",
    },
    {
      path: "dist/mainWindow/**/*.*",
    },
    {
      path: "dist/preload/**/*.*",
    },
  ],
  defaultCompression: "none",
};
