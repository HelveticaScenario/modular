const chokidar = require("chokidar");
const esbuild = require("esbuild");
async function run() {
  let result = await esbuild.build({
    entryPoints: ["src/main.ts", "src/renderer.tsx"],
    bundle: true,
    outdir: "./dist",
    platform: "node",
    sourcemap: "external",
    target: "es2019",
    incremental: true,
  });

  chokidar.watch("./src/**/*.ts").on("all", async (event, path) => {
    console.log("event", event);
    console.log("path", path);
    await result.rebuild();
  });
}

run();
