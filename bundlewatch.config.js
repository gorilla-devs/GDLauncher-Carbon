module.exports = {
  normalizeFilenames: /^.+?(\.\w+?)?\..+$/,
  files: [
    {
      path: "apps/desktop/dist/**/*.*",
    },
    {
      path: "packages/core_module/core.node",
    },
  ],
  defaultCompression: "none",
};
