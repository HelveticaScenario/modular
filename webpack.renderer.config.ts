import type { Configuration } from 'webpack';

import { rules } from './webpack.rules';
import { plugins } from './webpack.plugins';
import MonacoWebpackPlugin from 'monaco-editor-webpack-plugin';

rules.push({
  test: /\.css$/,
  use: [{ loader: 'style-loader' }, { loader: 'css-loader' }],
});

export const rendererConfig: Configuration = {
  module: {
    rules,
  },
  plugins: [
    ...plugins,
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
  // target: 'electron-renderer' // Target the renderer process
};
