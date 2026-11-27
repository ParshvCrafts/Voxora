import React from "react";
import { Composition, staticFile } from "remotion";
import { Film, FilmData } from "./Film";
import { Launch, LAUNCH_SECS } from "./Launch";

// Duration and size come from public/film.json, so the plan file is the single source of truth.
export const RemotionRoot: React.FC = () => (
  <>
  <Composition id="Launch" component={Launch} durationInFrames={Math.round(LAUNCH_SECS * 30)} fps={30} width={1080} height={1350} />
  <Composition
    id="Film"
    component={Film}
    durationInFrames={1}
    fps={30}
    width={1080}
    height={1350}
    defaultProps={{ data: {} as FilmData }}
    calculateMetadata={async () => {
      const data = (await (await fetch(staticFile("film.json"))).json()) as FilmData;
      const secs = data.scenes.reduce((a, s) => a + s.dur, 0);
      return { props: { data }, durationInFrames: Math.round(secs * data.fps), fps: data.fps, width: data.w, height: data.h };
    }}
  />

  </>
);
