/**
 * The API's errors as its callers read them (server/api/src/Http.php): every error answers
 * `{ error: { code, message, …details } }` with one HTTP status per code, and the two-step writes – an import over an
 * experience that exists, the removal of a scene or an entry – answer their first step as a 409 that carries what the step
 * shows the person and a confirmation to send back.
 */
import type { Confirmation } from "./editor";

/** What an error answer says: its code, its message and its other fields (`comparison` of a 409 confirm-import, `usages`, …) */
export interface ApiErrorFields {
  code: string;
  message: string;
  details: Record<string, unknown>;
}

/** An error answer's body (already parsed, or null when it was no JSON) as its fields; `unknown` without a code */
export function apiErrorFields(status: number, statusText: string, body: unknown): ApiErrorFields {
  const error = typeof body === "object" && body !== null && "error" in body ? (body as { error: unknown }).error : null;
  const { code, message, ...details } = typeof error === "object" && error !== null ? (error as Record<string, unknown>) : {};
  return {
    code: typeof code === "string" ? code : "unknown",
    message: typeof message === "string" && message !== "" ? message : `${status} ${statusText}`.trim(),
    details,
  };
}

/** An error as both callers' `ApiError` carries it */
export interface ApiErrorLike {
  status: number;
  code: string;
  details: Record<string, unknown>;
}

/** The two-step writes: the first step's code and the field of the details that holds what it shows */
export const TWO_STEPS = {
  import: { ask: "confirm-import", shows: "comparison" },
  removal: { ask: "confirm-delete", shows: "impact" },
} as const;

/** A first step to confirm: what it shows, the confirmation to send back, and whether an earlier one was void */
export interface ConfirmationStep<T> {
  shows: T;
  confirmation: Confirmation;
  /** 409 confirmation-void: the confirmation sent was expired, someone else's, or the experience changed since – this is the fresh step */
  changed: boolean;
}

/**
 * A 409 of a two-step write – its first step, or `confirmation-void` with the fresh one – as the step to confirm; any
 * other error, or an answer without the step's fields: null
 */
export function confirmationStep<T>(error: unknown, step: keyof typeof TWO_STEPS): ConfirmationStep<T> | null {
  if (typeof error !== "object" || error === null) return null;
  const { status, code, details } = error as Partial<ApiErrorLike>;
  if (status !== 409 || (code !== TWO_STEPS[step].ask && code !== "confirmation-void") || !details) return null;
  const shows = details[TWO_STEPS[step].shows] as T | undefined;
  const confirmation = details.confirmation as Confirmation | undefined;
  if (shows === undefined || !confirmation) return null;
  return { shows, confirmation, changed: code === "confirmation-void" };
}
