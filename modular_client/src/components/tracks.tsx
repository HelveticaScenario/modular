import React, { FC } from "react";
import { Track } from "./track";

export const Tracks: FC<{}> = () => {
  return (
    <div className="flex flex-row">
      <Track />
      <Track />
    </div>
  );
};
