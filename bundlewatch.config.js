module.exports = {
  normalizeFilenames: /^.+?(\.\w+?)?\..+$/,
  files: [
    {
      path: "apps/desktop/dist/**/*.*",
    },
    {
      path: "packages/native_interface/core.node",
    },
  ],
  defaultCompression: "none",
};
