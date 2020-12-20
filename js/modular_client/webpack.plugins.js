/* eslint-disable @typescript-eslint/no-var-requires */
const ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');
const { ESBuildPlugin } = require('esbuild-loader')

module.exports = [
  new ForkTsCheckerWebpackPlugin(),
  new ESBuildPlugin()
];
