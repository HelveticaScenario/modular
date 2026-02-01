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

import { mainConfig } from './webpack.main.config';
import { rendererConfig } from './webpack.renderer.config';

// if (process.env.APPLE_ID && process.env.APPLE_PASSWORD && process.env.APPLE_TEAM_ID) {

const config: ForgeConfig = {
  packagerConfig: {
    asar: true,
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
