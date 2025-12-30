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
      globalAPI: true
      // options:
      // languages: ['json', 'css', 'html', 'typescript'] // Specify only the languages you need to reduce bundle size
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
