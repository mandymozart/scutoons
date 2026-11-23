/**
 * The app's runtime error contract (RULES #16): every error that reaches the app carries an `ErrorCode`; the app maps a
 * code to its reaction and its own text. Low-level code throws typed errors (`ArSceneError`), the boundary maps them.
 */
export enum ErrorCode {
  /** The content needs a newer app (its schema MAJOR is ahead) */
  NOT_SUPPORTED = "not-supported",
  /** game.config.json fails the contract guard (shared/guards); problems in ErrorInfo.details */
  GAME_CONFIGURATION_INVALID = "game-configuration-invalid",
  /** The content could not be loaded (offline, not there, not JSON) */
  NETWORK_ERROR = "network-error",

  /** Stored progress of an older format was converted: parts of it may be missing */
  PROGRESS_CONVERTED = "progress-converted",
  /** Stored progress could not be read and starts afresh */
  PROGRESS_RESET = "progress-reset",

  /** The AR chunk (three.js and the scene) could not be loaded – offline */
  AR_LOAD_FAILED = "ar-load-failed",
  /** The camera or the tracking could not start (denied, unavailable, the tracking engine not loaded) */
  AR_START_FAILED = "ar-start-failed",
  /** The camera started but sends no picture */
  CAMERA_NOT_RESPONDING = "camera-not-responding",
  /** Anything else the scene could not do (building a scene) */
  AR_FAILED = "ar-failed",
}

/** Error or notice shown by the error overlay; `type` sets severity, `action` adds a button */
export interface ErrorInfo {
  code: ErrorCode;
  msg: string;
  type?: "critical" | "warning" | "info";
  /** The problems behind it, one line each (shown in development) */
  details?: string[];
  action?: {
    text: string;
    callback: () => void;
  };
}

export type ErrorListener = (error: ErrorInfo) => void;

/** The codes an AR scene reports with its `error` status */
export type ArErrorCode = ErrorCode.AR_LOAD_FAILED | ErrorCode.AR_START_FAILED | ErrorCode.CAMERA_NOT_RESPONDING | ErrorCode.AR_FAILED;

/** Why the AR scene failed (`status` "error"): the code the app reacts to, and the technical message (console, debug overlay) */
export interface ArError {
  code: ArErrorCode;
  message: string;
}

/** An AR failure with its code, thrown by the scene and its tracker; the scene reports it as an `ArError` */
export class ArSceneError extends Error {
  constructor(readonly code: ArErrorCode, message: string) {
    super(message);
    this.name = "ArSceneError";
  }
}
