import type { Configuration } from 'webpack';

import { rules } from './webpack.rules';
import MonacoWebpackPlugin from 'monaco-editor-webpack-plugin';

// eslint-disable-next-line @typescript-eslint/no-var-requires
const ForkTsCheckerWebpackPlugin: typeof import('fork-ts-checker-webpack-plugin') = require('fork-ts-checker-webpack-plugin');

rules.push({
  test: /\.css$/,
  use: [{ loader: 'style-loader' }, { loader: 'css-loader' }],
});

export const rendererConfig: Configuration = {
  module: {
    rules,
  },
  plugins: [
    new ForkTsCheckerWebpackPlugin({
      logger: 'webpack-infrastructure',
      typescript: {
        configFile: 'src/renderer/tsconfig.json',
        build: true,
      },
    }),
    new MonacoWebpackPlugin({
      globalAPI: true,
      languages: ['javascript', 'typescript', 'json', 'css']
    })
  ],
  resolve: {
    extensions: ['.js', '.ts', '.jsx', '.tsx', '.css'],
  },
  node: {
    __dirname: false,
    __filename: false,
  },
};
