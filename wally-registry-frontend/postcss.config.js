module.exports = {
  modules: true,
  plugins: {
    "postcss-modules": {
      globalModulePaths: [
        // Put your global css file paths.
        /fonts/,
      ],
    },
  },
}
