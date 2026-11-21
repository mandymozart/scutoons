import type { ExperienceData, RecognitionMode, TargetData, TargetRecognitionData } from "../types/index";

/**
 * Which recognition data the reader gives the tracker for a target (#306, content schema 4.2) – pure, no storage:
 * - the **experience** says `"original"` (none = the default: every image target is made from its printed image on the
 *   device, the path tested with every target) or `"prepared"` (a target's frame made ahead where it has one);
 * - a **device** may override it (the debug overlay's switch, `utils/recognition-setting.ts`) to compare both on one
 *   page without publishing;
 * - a **target** with `recognitionOff: true` always uses its printed image.
 */

/** The experience's mode: `"prepared"` only when it says so – none, or a text this app does not know, is `"original"` */
export const recognitionModeOf = (experience: Pick<ExperienceData, "recognition"> | null | undefined): RecognitionMode =>
  experience?.recognition === "prepared" ? "prepared" : "original";

/** The mode in force: the device's override, else the experience's */
export const resolveRecognitionMode = (experience: Pick<ExperienceData, "recognition"> | null | undefined, device: RecognitionMode | null): RecognitionMode =>
  device ?? recognitionModeOf(experience);

/** The target's recognition data to use under `mode`: its frame made ahead, or undefined – the printed image then */
export const recognitionFor = (target: Pick<TargetData, "recognition" | "recognitionOff">, mode: RecognitionMode): TargetRecognitionData | undefined =>
  mode === "prepared" && target.recognitionOff !== true ? target.recognition : undefined;
