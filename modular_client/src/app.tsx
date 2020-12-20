import React from "react";
import { FC } from "react";
import { Canvas } from "./components/canvas";
import { Command } from "./components/command";
import { Tracks } from "./components/tracks";

export const App: FC<{}> = () => {
  return (
    <div className="text-white flex-col flex h-screen w-screen">
      <Tracks />
      <Canvas />
      <Command />
    </div>
  );
};
