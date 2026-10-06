/** The panel's texts, from the app's own messages (each app has its paraglide catalog) */
export interface AskLabels {
	title: string;
	history: string;
	historyUnavailable: string;
	newChat: string;
	close: string;
	empty: string;
	you: string;
	agent: string;
	working: string;
	/** the line above the field: where the person is (`page`) and what the agent may do */
	context: (page: string) => string;
	placeholder: string;
	send: string;
	stop: string;
	resize: string;
	unavailable: string;
	signedOut: string;
	/** the link text of a saved draft (creator) */
	draft: (id: string) => string;
}
