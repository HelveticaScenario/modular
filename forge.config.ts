import type { ForgeConfig } from '@electron-forge/shared-types';
import { MakerSquirrel } from '@electron-forge/maker-squirrel';
import { MakerZIP } from '@electron-forge/maker-zip';
import { MakerDeb } from '@electron-forge/maker-deb';
import { MakerRpm } from '@electron-forge/maker-rpm';
// import { MakerFlatpak } from '@electron-forge/maker-flatpak';
import { PublisherGithub } from '@electron-forge/publisher-github';
import { AutoUnpackNativesPlugin } from '@electron-forge/plugin-auto-unpack-natives';
import { WebpackPlugin } from '@electron-forge/plugin-webpack';
import { FusesPlugin } from '@electron-forge/plugin-fuses';
import { FuseV1Options, FuseVersion } from '@electron/fuses';
import * as fs from 'fs';
import * as path from 'path';

import { mainConfig } from './webpack.main.config';
import { rendererConfig } from './webpack.renderer.config';

// if (process.env.APPLE_ID && process.env.APPLE_PASSWORD && process.env.APPLE_TEAM_ID) {

const config: ForgeConfig = {
  packagerConfig: {
    asar: {
      unpack: '**/*.node'
    },
    executableName: 'Switchboard',
    osxSign: {
      identity: 'Developer ID Application: Daniel Lewis (HA98TTLCR7)',
      optionsForFile: () => {
        return {
          hardenedRuntime: true,
          entitlements: 'entitlements.plist',
          'entitlements-inherit': 'entitlements.plist',
          signatureFlags: 'library'
        };
      }
    },

    // macOS notarization configuration
    // Only runs when environment variables are present (i.e., in CI)
    osxNotarize: process.env.APPLE_ID && process.env.APPLE_PASSWORD && process.env.APPLE_TEAM_ID ? {
      appleId: process.env.APPLE_ID,
      appleIdPassword: process.env.APPLE_PASSWORD,
      teamId: process.env.APPLE_TEAM_ID
    } : undefined
  },
  rebuildConfig: {},
  hooks: {
    // Copy @modular/core workspace package to node_modules before packaging
    // This is needed because yarn workspaces use symlinks which don't survive packaging
    packageAfterCopy: async (_config, buildPath) => {
      const sourceDir = path.join(__dirname, 'crates', 'modular');
      const targetDir = path.join(buildPath, 'node_modules', '@modular', 'core');
      
      // Ensure target directory exists
      fs.mkdirSync(targetDir, { recursive: true });
      
      // Files to copy from the native module package
      const filesToCopy = [
        'index.js',
        'index.d.ts',
        'package.json',
      ];
      
      for (const file of filesToCopy) {
        const src = path.join(sourceDir, file);
        const dest = path.join(targetDir, file);
        if (fs.existsSync(src)) {
          fs.copyFileSync(src, dest);
        }
      }
      
      // Copy the native .node file for the current platform
      const nodeFiles = fs.readdirSync(sourceDir).filter(f => f.endsWith('.node'));
      for (const nodeFile of nodeFiles) {
        fs.copyFileSync(
          path.join(sourceDir, nodeFile),
          path.join(targetDir, nodeFile)
        );
      }
    }
  },
  makers: [
    new MakerSquirrel({
      name: 'Switchboard'
    }),
    new MakerZIP({}, ['darwin']),
    new MakerRpm({
      options: {
        bin: 'Switchboard',
      },
    }),
    new MakerDeb({
      options: {
        bin: 'Switchboard',
      },
    }),
    // new MakerFlatpak({
    //   // @ts-ignore
    //   options: {
    //     bin: 'Switchboard',
    //     id: 'com.helveticascenario.switchboard',
    //   },
    // }),
  ],
  publishers: [
    new PublisherGithub({
      repository: {
        owner: 'HelveticaScenario',
        name: 'modular',
      },
      prerelease: false,
      draft: false,
    }),
  ],
  plugins: [
    new AutoUnpackNativesPlugin({}),
    new WebpackPlugin({
      mainConfig,
      renderer: {
        config: rendererConfig,
        entryPoints: [
          {
            html: './src/index.html',
            js: './src/renderer.tsx',
            name: 'main_window',
            preload: {
              js: './src/preload.ts',
            },
          },
        ],
      },
    }),
    // Fuses are used to enable/disable various Electron functionality
    // at package time, before code signing the application
    new FusesPlugin({
      version: FuseVersion.V1,
      [FuseV1Options.RunAsNode]: false,
      [FuseV1Options.EnableCookieEncryption]: true,
      [FuseV1Options.EnableNodeOptionsEnvironmentVariable]: false,
      [FuseV1Options.EnableNodeCliInspectArguments]: false,
      [FuseV1Options.EnableEmbeddedAsarIntegrityValidation]: true,
      [FuseV1Options.OnlyLoadAppFromAsar]: true,
    }),
  ],
};

export default config;
