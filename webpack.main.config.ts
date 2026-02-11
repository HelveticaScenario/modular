import type { Configuration } from 'webpack';

import { rules } from './webpack.rules';

// eslint-disable-next-line @typescript-eslint/no-var-requires
const ForkTsCheckerWebpackPlugin: typeof import('fork-ts-checker-webpack-plugin') = require('fork-ts-checker-webpack-plugin');

export const mainConfig: Configuration = {
    /**
     * This is the main entry point for your application, it's the first file
     * that runs in the main process.
     */
    entry: './src/main/main.ts',
    // Put your normal webpack config below here
    externals: {
        '@modular/core': 'commonjs @modular/core',
    },
    module: {
        rules: [
            ...rules,
            {
                test: /\.(m?js|node)$/,
                parser: { amd: false },
                use: {
                    loader: '@vercel/webpack-asset-relocator-loader',
                    options: {
                        outputAssetBase: 'native_modules',
                    },
                },
            },
        ],
    },
    plugins: [
        new ForkTsCheckerWebpackPlugin({
            logger: 'webpack-infrastructure',
            typescript: {
                configFile: 'src/main/tsconfig.json',
                build: true,
            },
        }),
    ],
    resolve: {
        extensions: ['.js', '.ts', '.jsx', '.tsx', '.css', '.json'],
    },
};
