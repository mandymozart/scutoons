// Failures and problems as sentences in the app's language (messages.ts). Pure apart from the locale.
import { m } from './messages.ts';
import type { FileProblem, NameProblem } from './files.ts';
import { LIMITS } from './files.ts';

type Params = Record<string, string | number | null | undefined>;

/** why a request did not work, one muted sentence (a 403 is not an error toast) */
export function failureText(f: { reason: string; code: string; message: string }): string {
	if (f.reason === 'creator_only') return m['designer.reason_creator_only']();
	if (f.reason === 'signed_out') return m['designer.reason_signed_out']();
	if (f.reason === 'unavailable') {
		if (f.code === 'live_show') return m['designer.reason_live_show']();
		if (f.code === 'build_busy') return m['designer.reason_build_busy']();
		if (f.code === 'no_build_service') return m['designer.reason_no_build_service']();
		return m['designer.reason_build_unavailable']();
	}
	return m['designer.reason_error']({ message: f.message });
}

/** the problems that are the user's alone (not creator, not signed in): the actions are disabled, nothing is an error */
export const isQuiet = (reason: string): boolean => reason === 'creator_only' || reason === 'signed_out';

export function problemText(p: FileProblem): string {
	const params: Params = { file: p.file, limit: p.limit, actual: p.actual };
	switch (p.code) {
		case 'empty':
			return m['designer.problem_empty']();
		case 'no_main':
			return m['designer.problem_no_main']();
		case 'too_many':
			return m['designer.problem_too_many'](params);
		case 'file_too_large':
			return m['designer.problem_file_too_large'](params);
		case 'total_too_large':
			return m['designer.problem_total_too_large'](params);
		case 'bad_name':
			return m['designer.problem_bad_name'](params);
	}
}

export function nameProblemText(p: NameProblem): string {
	switch (p) {
		case 'bad_name':
			return m['designer.name_bad_name']();
		case 'exists':
			return m['designer.name_exists']();
		case 'main_fixed':
			return m['designer.name_main_fixed']();
		case 'too_many':
			return m['designer.name_too_many']({ limit: LIMITS.maxFiles });
	}
}
