import type { ExperienceData, RecognitionMode, TargetData, TargetRecognitionData } from "../types/index";
import { getExperience } from "../content/game-config";
import { recognitionFor, recognitionModeOf, resolveRecognitionMode } from "../scene/recognition-mode";
import { readSwitch, writeSwitch } from "./device-storage";

/**
 * The recognition path on this device (#306): the experience's setting (`experience.recognition`), unless the debug
 * overlay's switch overrides it – stored on this device (`device-storage.ts`), so a tester compares Original and Prepared
 * on the same page without publishing. The tracker holds its image targets per session: a change applies after a reload.
 */
const KEY = "recognition";

/** The device's override, null = the experience decides */
export const getRecognitionOverride = (): RecognitionMode | null => {
  const stored = readSwitch(KEY);
  return stored === "original" || stored === "prepared" ? stored : null;
};

/** Store the override (null = back to the experience's setting) */
export const setRecognitionOverride = (mode: RecognitionMode | null): void => writeSwitch(KEY, mode);

/** The loaded experience, null before the content is loaded */
const loadedExperience = (): ExperienceData | null => {
  try {
    return getExperience() ?? null;
  } catch {
    return null;
  }
};

/** The experience's own setting (the loaded content), regardless of an override */
export const getExperienceRecognitionMode = (): RecognitionMode => recognitionModeOf(loadedExperience());

/** The mode in force on this device */
export const getRecognitionMode = (): RecognitionMode => resolveRecognitionMode(loadedExperience(), getRecognitionOverride());

/** The target's frame made ahead when the mode in force uses it, else undefined (its printed image) */
export const targetRecognition = (target: Pick<TargetData, "recognition" | "recognitionOff">): TargetRecognitionData | undefined =>
  recognitionFor(target, getRecognitionMode());

/** For reports: the mode in force and where it comes from */
export const recognitionLabel = (): string => {
  const override = getRecognitionOverride();
  return override
    ? `${override} (this device's override – the experience says ${getExperienceRecognitionMode()})`
    : `${getRecognitionMode()} (the experience's setting)`;
};
