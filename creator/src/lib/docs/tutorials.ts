// The creators' use cases and their tutorials (/docs). The list was reviewed by the owner on #624; a case without steps
// is listed in the index as a guide still to write. Steps: one or two sentences, UI labels in “quotes”, as the app says them.
import type { Group } from './catalog.ts';

export const GROUPS: Group[] = [
	{
		key: 'start',
		title: 'Getting started',
		intro: 'Signing in, finding your way, and connecting the tools around the creator: your own Claude, the deck editor, a deck.',
		cases: [
			{
				slug: 'sign-in',
				title: 'Sign in and find your way',
				summary: 'Sign in to the creator and get to your toon',
				who: ['all'],
				where: ['creator'],
				often: 3,
				facts: [
					{
						label: 'Already signed in',
						text: 'Signed in on scutoons.com means signed in here too.'
					}
				],
				steps: [
					{
						text: 'Open creator.scutoons.com. Under “Creator sign-in” type your email and press “Send code”.'
					},
					{
						text: 'Type the “One-time code” from the mail and press “Sign in” – or just tap the link in the mail.'
					},
					{
						text: '“No access” means your account has no creator role yet: ask your lead, or try another email with “Switch account”.'
					},
					{
						text: 'The start page shows “Continue” (where you were last), your “Toons”, what is “Not live yet” and what is “Waiting for you”.'
					},
					{
						text: 'Ctrl+K (or /) opens “Search or jump to …”: type a toon, a chapter or a page and press Enter. Alt+↑ and Alt+↓ go a level up or down.'
					}
				],
				note: 'Accounts are made in the reader on scutoons.com; the creator opens for accounts with the creator role.'
			},
			{
				slug: 'install-app',
				title: 'Install the creator as an app',
				summary: 'The creator on your home screen, with notifications',
				who: ['all'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'languages',
				title: 'Interface language and text language',
				summary: 'The language you work in versus the language you write in',
				who: ['all'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'connect-claude',
				title: 'Connect your own Claude',
				summary: 'Your Claude saves drafts; you check and publish them',
				who: ['all'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'editor-sign-in',
				title: 'Install and sign in to the deck editor',
				summary: 'The editor reads and saves your drafts',
				who: ['performer', 'sound'],
				where: ['editor'],
				often: 1
			},
			{
				slug: 'pair-deck',
				title: 'Pair a deck with your account',
				summary: 'A code on the deck, approved in your account',
				who: ['performer'],
				where: ['deck'],
				often: 1
			}
		]
	},
	{
		key: 'toon',
		title: 'Toon, seasons, chapters',
		intro: 'The frame of a story: the toon, its seasons and chapters, the screenplay and the Kanon behind it.',
		cases: [
			{
				slug: 'toon-settings',
				title: 'Toon settings',
				summary: 'Title, description, languages, avatar, trailer, status per language',
				who: ['writer', 'lead'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'new-season',
				title: 'A new season and its cover',
				summary: 'Create a season, order its chapters, letter its cover',
				who: ['writer', 'lead'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'new-chapter',
				title: 'A new chapter',
				summary: 'Create a chapter and set its title, date, key visual and status',
				who: ['writer'],
				where: ['creator'],
				often: 2,
				steps: [
					{
						text: 'Open your toon from “Toons” (or with Ctrl+K): the last row of its chapter list is “+ New chapter”.'
					},
					{
						text: '“+ New chapter” takes the next free number, adds the chapter to its season as a draft and opens it. Readers see nothing yet.'
					},
					{
						text: 'In the tree on the left choose “Meta” and fill in “Title”, “Description”, “Release” and “Author” (the author is required).'
					},
					{
						text: 'Drop pictures on the “Thumbnail” and “Key visual” slots, and add who is “Featuring” – they must be in the season’s cast.'
					},
					{
						text: '“Status” is set per language: “published”, “draft” or “coming soon” (listed, not readable yet). It is kept in the season.'
					},
					{
						text: '“Save” keeps it all as a draft and asks for a short note. Ctrl+S saves at once, without asking.'
					}
				],
				note: 'The chapter number is its address and never changes. If “+ New chapter” is grey, save the toon’s language list first.'
			},
			{
				slug: 'screenplay',
				title: 'Write the screenplay',
				summary: 'One screenplay per toon, in the screenplay format',
				who: ['writer'],
				where: ['creator'],
				often: 2,
				steps: [
					{ text: 'Open your toon’s “Script” tab: the Kanon, one screenplay per season and one per chapter, each “published”, “draft” or “not written”.' },
					{ text: 'Open one: it is read-only at first – “Edit script” unlocks it.' },
					{
						text: 'Write in the screenplay format: the spaces count – action at 4, the character’s name in capitals at 25–30, dialogue at 16. Tab puts in 4 spaces.'
					},
					{ text: 'The “Outline” on the side lists the scenes; “Format rules” opens all the rules.' },
					{ text: 'Save and publish as everywhere else. A translation may be partial: the source language is the master.' }
				],
				note: 'The screenplay is for the team – readers never see it.'
			},
			{
				slug: 'kanon',
				title: 'Write the Kanon',
				summary: 'The story text behind a toon, a character or a place',
				who: ['writer'],
				where: ['creator'],
				often: 2,
				steps: [
					{ text: 'Open the Kanon from the toon’s “Script” tab, a place’s “Kanon” link on the map, or the “Kanon” square of a character, persona or look.' },
					{ text: 'An empty one offers “Start empty”, “Start with the headings” or – in a translation – a copy of the source.' },
					{ text: 'Write in sections: “## heading” opens one. Sections are optional and their order is fixed – the rules are in the “Info” panel.' },
					{ text: 'Save drafts as you go. Publishing makes it Kanon: the generators, the artist briefs and the AI connection read the published text.' }
				],
				note: 'The Kanon is the team’s story text: readers never see it.'
			},
			{
				slug: 'story-flow',
				title: 'Check the story flow',
				summary: 'Where readers go, and where they get stuck',
				who: ['writer', 'lead'],
				where: ['creator'],
				often: 2,
				steps: [
					{
						text: 'On the toon page the round flow button opens the toon’s “Flow”: seasons, chapters, map and jumps. In the chapter editor the same button shows this chapter’s story flow.'
					},
					{ text: 'It is read-only: a box opens its chapter, so you fix things where they are.' },
					{
						text: 'The chips show the trouble: “may get stuck” (every choice has a condition – give one none, like “Go on”), “unreachable”, “endless loop” or “never true”.'
					},
					{ text: '“Unused” lists variables set but never read, read but never set, and points nothing jumps to.' },
					{ text: '“Flow” in the menu shows every toon and the jumps between them.' }
				]
			}
		]
	},
	{
		key: 'characters',
		title: 'Characters, cast, credits',
		intro: 'The characters every toon shares, who appears in a season, and who made it.',
		cases: [
			{
				slug: 'new-character',
				title: 'A new character',
				summary: 'Import a character’s pictures and fill in who they are',
				who: ['artist', 'writer'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'character-looks',
				title: 'Looks, expressions and moods',
				summary: 'Looks, expressions, A-poses, transitions, the 3D model',
				who: ['artist'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'wanted-images',
				title: 'Brief an artist or an AI',
				summary: 'The pictures still missing, with the prompts to make them',
				who: ['artist'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'cast-credits',
				title: 'Cast and credits',
				summary: 'Who appears in a season, and who made it',
				who: ['writer', 'lead'],
				where: ['creator'],
				often: 1
			}
		]
	},
	{
		key: 'pages',
		title: 'Chapter pages',
		intro: 'A chapter is read slice by slice: the pictures, the lettering, the sound and what readers can do.',
		cases: [
			{
				slug: 'import-slices',
				title: 'Import slices',
				summary: 'Bring a chapter’s pictures in, in bulk, and order them in groups',
				who: ['artist'],
				where: ['creator'],
				often: 3,
				facts: [
					{
						label: 'Size',
						text: 'A slice is 390 px wide at 1x; upload 1170 px wide (3x) and the 2x and 1x are made from it.'
					}
				],
				steps: [
					{
						text: 'In the chapter tree select a group (or “Add group” at the bottom for a new one): its slices show as a strip.'
					},
					{
						text: '“Import slices” under the strip lets you choose several images at once (JPEG, PNG or WebP).'
					},
					{
						text: '“Import n images” shows each with its target: “New slice” or “Replace n · name”. A file named like slice-26 or 26 goes to slice 26.'
					},
					{
						text: 'Choose where new slices go (“after slice n”, “before slice n”, “at the end”), then press “Import · … replace, … new”.'
					},
					{
						text: 'In the group’s panel choose its “Layout”: “Vertical” (read top to bottom), “Horizontal” (side by side as you scroll) or “Endless” (a band readers drag).'
					}
				],
				note: 'A replaced slice keeps its texts, items, events and audio; its old picture stays as an earlier version. One image at a time: drop it on the middle of a slice to replace it, on its top or bottom edge to insert it.'
			},
			{
				slug: 'slice-media',
				title: 'A video or picture background',
				summary: 'A slice’s background, its fit and poster, and a content warning',
				who: ['artist'],
				where: ['creator'],
				often: 2,
				steps: [
					{ text: 'Select a slice and open the “Slice” tab in the inspector.' },
					{ text: 'Fill the “Image” slot. For a video, fill the “Video” slot too – the image becomes its poster.' },
					{ text: 'For a video choose “Loop”, “Autoplay” and “Controls”.' },
					{
						text: 'Choose the “Placement” (“In the reading column” or “Full viewport”), the “Aspect ratio” and how it should “Fit” – the previews show a phone and a desktop.'
					},
					{ text: 'Tick “Sensitive content” if needed: readers who hide sensitive content see it blurred and muted.' }
				],
				note: 'A poster with another aspect ratio than its video makes the slice shift while it loads – the inspector warns about it.'
			},
			{
				slug: 'lettering',
				title: 'Letter a slice',
				summary: 'Text boxes: preset, font, paint, tail, how they come in',
				who: ['artist', 'writer'],
				where: ['creator'],
				often: 3,
				steps: [
					{
						text: 'Select a slice, open the “Layers” tab in the inspector and press “+ Add text box”.'
					},
					{
						text: 'Pick a preset card – Narration, Speech, Thought, Onomatopoeia, Story, Scene or Lyrics. Or take the “Text box” tool in the canvas toolbar and drag on the slice.'
					},
					{
						text: 'Type the text. For a bubble choose a “Tail” (“Speech (wedge)” or “Thought (circles)”) and drag its tip and base on the slice.'
					},
					{
						text: '“Advanced” holds the “Anchor”, “Font”, “Text size”, “Colour” (solid or a gradient) and the “Outline”.'
					},
					{
						text: 'To let it come in later, add an event in the “Events” tab that shows the text box and choose “How it comes in”: Appear, Pop, Slam, Type on and more.'
					},
					{
						text: 'Switch the text language in the top bar and type the translation. Position and size are the same in every language; only the text changes.'
					}
				]
			},
			{
				slug: 'translate',
				title: 'Translate a chapter',
				summary: 'Copy from the source language, then check what is missing',
				who: ['writer'],
				where: ['creator'],
				often: 2,
				steps: [
					{ text: 'On the toon page, under “Languages”, “+ Add” adds a translation. Save the draft before the editors offer the new language.' },
					{
						text: 'Switch the text language with the globe select in the top bar: the source is marked ★, and the button turns dark while you see a translation.'
					},
					{ text: 'Type the translation into each text field; the original shows under it. On a phone, “Copy the original” fills it in to start from.' },
					{ text: 'For a script or Kanon text the empty language offers “Start empty” or “Copy the … script”.' },
					{ text: 'Check the “Problems”: a missing translation and a text much longer than the original (over 130 %) are warnings.' }
				],
				note: 'The source language is changed on the script page (“Make … the source”), after the script is saved.'
			},
			{
				slug: 'choices-events',
				title: 'Choices and events',
				summary: 'Visual-novel events, choices, conditions and jumps',
				who: ['writer'],
				where: ['creator'],
				often: 2,
				steps: [
					{
						text: 'Select a slice and open the “Events” tab, or draw an area with the “Event” tool for a tap event. For a whole group: its “Visual novel” section → “+ VN script”.'
					},
					{ text: 'Add events with the buttons: “+ showMessage” for dialog, “+ addChoice” and “+ showChoices” for choices, “+ jumpTo” to go elsewhere.' },
					{ text: 'A choice has a “Text” and what happens “When picked”: set a variable, give an item, jump to a group, continue at a point.' },
					{ text: 'Points are “+ label” events; “+ divert” and choices continue there.' },
					{ text: '“+ Only if” puts a condition on any event: “Simple” (variable, comparison, value) or an “Expression” like count("e…") < 3.' },
					{ text: 'Watch the chips: “may get stuck”, “unreachable” or “endless loop” say where readers could not go on.' }
				],
				note: 'A speaker who is not on stage is skipped – the fix button “+ joinScene before” brings them in.'
			},
			{
				slug: 'variables',
				title: 'Variables in text',
				summary: 'The reader’s name and other values inside a text',
				who: ['writer'],
				where: ['creator'],
				often: 2,
				steps: [
					{ text: 'In a text field type “{”: the hint says “Enter = builder”. Press Enter for the builder, or keep typing the code.' },
					{ text: 'In the builder choose “What”: a “Value”, “If … else”, “Per visit” or “Code”, then search the value – story variables and functions.' },
					{
						text: '“Apply” puts it into the text as a chip. Click the chip to change it in the builder, double click to edit its code; Backspace removes it whole.'
					},
					{ text: 'On the canvas “Preview texts with expressions” shows the text “as written”, for a “new guest”, “halfway” or with “everything collected”.' }
				],
				note: 'A brace as text needs a backslash before it. And = sets a value: to compare, write ==.'
			},
			{
				slug: 'chapter-sound',
				title: 'Music and samples in a chapter',
				summary: 'Music volumes per slice, samples on scroll or on a click',
				who: ['sound'],
				where: ['creator'],
				often: 2,
				steps: [
					{
						text: 'In the chapter tree choose “Audio”. Under “Music” add tracks with “+ Track” (an “ID” and a “File”), and set the “Default track” and the “Crossfade (ms)”.'
					},
					{ text: 'Under “Sample library” add each sound with “+ Sample”: an “ID” and a “File”.' },
					{
						text: 'Select a slice and open its “Audio” tab: the “Music” faders set each track’s volume from this slice on. “×” goes back to the earlier level.'
					},
					{ text: 'Under “Samples” add “+ on scroll” (it plays when the slice comes into view) or “+ on click (area)”, and choose its “Sound”.' },
					{ text: 'Set “Volume”, “Delay (ms)”, “Loop” and the rest per sample. Draw a click area with the “Sample area” tool on the slice.' }
				],
				note: 'Music and samples are shared by every language. File paths are relative to the toon’s media folder.'
			}
		]
	},
	{
		key: 'storyboard',
		title: 'Storyboard',
		intro: 'From the screenplay to shots: blocking in 3D, generated variants, a camera in your hand.',
		cases: [
			{
				slug: 'storyboard',
				title: 'Storyboard a chapter',
				summary: 'Script into shots, blocking in 3D, the timeline and the output',
				who: ['artist', 'writer'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'generate-variants',
				title: 'Generate and choose variants',
				summary: 'Generate pictures (the cost first), choose, compare',
				who: ['artist'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'handheld-camera',
				title: 'The handheld camera',
				summary: 'Frame a shot by hand with your phone',
				who: ['artist'],
				where: ['creator'],
				often: 1
			}
		]
	},
	{
		key: 'world',
		title: 'Items, carrier bag, map',
		intro: 'What readers collect in their carrier bag, and the world map they travel.',
		cases: [
			{
				slug: 'catalog-item',
				title: 'A new item',
				summary: 'An item, recipe, album or product in the catalog',
				who: ['writer', 'artist'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'place-item',
				title: 'Give readers an item',
				summary: 'Place an item in a slice, and let it come back',
				who: ['writer'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'score-collage',
				title: 'A score collage',
				summary: 'Shapes and characters on a score’s bars',
				who: ['artist'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'world-map',
				title: 'Build the world map',
				summary: 'Places, regions, routes, terrain and place pictures',
				who: ['artist', 'writer'],
				where: ['creator'],
				often: 1
			},
			{
				slug: 'unlock-place',
				title: 'Unlock a place from the story',
				summary: 'A place opens when readers reach a point in the story',
				who: ['writer'],
				where: ['creator'],
				often: 2
			}
		]
	},
	{
		key: 'sound',
		title: 'Sound design: rigs and modules',
		intro: 'Every character has a sound rig: modules and instruments, wired and mixed, heard the way a deck plays it.',
		cases: [
			{
				slug: 'character-rig',
				title: 'A character’s rig in the creator',
				summary: 'Start from modules, set parameters, listen, assign',
				who: ['sound'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'audio-setup',
				title: 'Set up your sound card',
				summary: 'Driver, device, buffer, channels and test tones',
				who: ['performer', 'sound'],
				where: ['editor', 'deck'],
				often: 1,
				facts: [
					{ label: 'On a deck', text: 'deck --audio cpal --audio-host asio --audio-device fireface --audio-buffer 128 (--audio-list shows the devices).' }
				],
				steps: [
					{ text: 'The menu (top left) → “Audio & inputs” opens the “Audio” panel.' },
					{ text: 'Under “HOST” choose the driver: ASIO for an RME and other studio cards, PipeWire on a deck, WASAPI otherwise. Then pick the device.' },
					{ text: 'Under “BUFFER” choose a size; smaller is quicker but harder for the computer. With ASIO the size is set in the driver’s own dialog.' },
					{
						text: '“GROUPS ON CHANNELS”: click a cell to put a group of the rig’s outlets on a channel. A channel number or a group’s name plays a test tone; click again to stop it.'
					},
					{ text: '“Apply” writes the settings and restarts the stage’s sound on them.' }
				],
				note: 'Under WASAPI the buffer only grows (an RME gets about 1056 frames): for small buffers use ASIO.'
			},
			{
				slug: 'open-rig',
				title: 'Open a rig and hear it',
				summary: 'A character’s rig on the stage, played as a deck plays it',
				who: ['sound'],
				where: ['editor'],
				often: 3,
				facts: [
					{
						label: 'Start it',
						text: 'cargo run -p deck-editor in the deck folder; a deck with the full image has it built in.'
					}
				],
				steps: [
					{
						text: 'Start the deck editor: it opens in “Build” with a rig on the stage.'
					},
					{
						text: 'Sign it in once: point at the account icon (top right) for its code and approve it on scutoons.com under Account → “Pair a deck”. The editor then reads and saves drafts; publishing stays in the creator.'
					},
					{
						text: 'In the “Inspector” click the rig’s name at the top: the list shows your toon’s rigs – pick one.'
					},
					{
						text: 'The editor starts in the wiring. The play button beside the BPM (or W) plays the rig as a deck plays it.'
					},
					{
						text: 'Drag a knob to turn it, anywhere else to orbit. Shift + drag pans, the wheel zooms, F frames everything.'
					},
					{
						text: 'There is no save button: the rig saves itself as a draft (Ctrl+S saves at once).'
					}
				],
				note: 'The approval code changes every 10 minutes, and only creator accounts can approve it.'
			},
			{
				slug: 'wire-rig',
				title: 'Wire a rig',
				summary: 'Tracks and nodes, ports and cables, far-world addresses',
				who: ['sound'],
				where: ['editor'],
				often: 2,
				steps: [
					{ text: 'Build starts in the wiring: every track is one body with its outlets. Double click a body (or point at it and press Tab) to go in.' },
					{ text: 'Drag from an out to an in to connect them. Pull a cable off an in and let go to cut it, or drop it on another in to move it.' },
					{ text: 'Right click a port (or the stage) and “type to find”: it lists the ports that one can reach.' },
					{
						text: '“Add” beside the rig picker adds a node inside a track, a “world · far” node with its holder from the address book, or at the rig a track playing an instrument or a module.'
					},
					{ text: '“Remove” (or Delete) takes the selected node or track away with its cables; Ctrl+Z takes any change back.' },
					{ text: 'The crumbs on top lead back: “rig” shows every track as one body again. Esc or Shift+Tab goes one level up.' }
				],
				note: 'At the rig level Esc quits the editor. Audio can’t go into a control in.'
			},
			{
				slug: 'mixer',
				title: 'Mix a rig',
				summary: 'Faders, gains inside a track, pan, mute and peaks',
				who: ['sound'],
				where: ['editor'],
				often: 2,
				steps: [
					{ text: 'Open the “Mixer” panel (bottom right): one strip per track at the level shown, plus the “out” strip.' },
					{ text: 'The pills on top choose the level: “rig” for the tracks, or a track you entered for the gains inside it.' },
					{ text: 'Drag a fader – it goes into the draft when you let go. Double click the dB value to type one, Enter sets it, Esc leaves it.' },
					{ text: 'Drag the pan bar for left and right; “M” mutes the track.' },
					{ text: 'The peak readout holds the loudest moment, red once it went over 0 dB. Click it to clear it.' }
				],
				note: 'A fader fed by a cable has no handle: the cable sets it.'
			},
			{
				slug: 'module-params',
				title: 'A module’s parameters by page',
				summary: 'Areas and pages, eight knobs a page, the Analog Four way',
				who: ['sound'],
				where: ['editor'],
				often: 3,
				steps: [
					{ text: 'On the stage a module shows one ring cluster per area, like “SRC · 4”. A click opens it in place, another click closes it.' },
					{ text: 'Select the node itself (not its track): the “Inspector” shows “NODE”, then “PARAMETERS” with a row of area pills.' },
					{
						text: 'Press an area – SRC, FLTR, AMP, LFO, VOICE on the analog synth – to open its first page. Press it again for its next page; it wraps round.'
					},
					{
						text: 'Each page is two rows of four knobs: drag or use the wheel to turn one in the engine. Twin pages put the same idea on the same knob, like the level of OSC 1 and OSC 2.'
					},
					{ text: 'Parameters that are on no page follow below as bars, then the “PORTS”.' }
				],
				note: 'A parameter fed by a cable shows “< source” and can’t be turned by hand. To open a page straight away: --select NODE --page "SRC/OSC 2".'
			},
			{
				slug: 'own-module',
				title: 'Write your own module',
				summary: 'Write, build and hear a module; save it as a draft',
				who: ['sound'],
				where: ['editor'],
				often: 1,
				steps: [
					{ text: 'Switch to “Module” and pick a module in the “Library”. A rig without one offers “New module” for its character.' },
					{ text: '“Open in editor” opens the module’s code in your code editor (VS Code, or the command in DECK_CODE_EDITOR).' },
					{
						text: 'Press “Build”. When it works the parameters read “heard on the stage”; if not, the “Problems” panel lists them and a click opens the code there.'
					},
					{ text: 'Turn its parameters and listen. The working copy saves itself as a draft whenever its sources change.' },
					{ text: 'Back in “Build”, add it to a rig as a track: “Add” → module:<name>.' }
				],
				note: 'A factory module is read-only: it saves as a module of your own, my_<id>.'
			},
			{
				slug: 'library',
				title: 'The library',
				summary: 'Find modules, instruments and pictures and drag them in',
				who: ['sound', 'artist'],
				where: ['editor'],
				often: 2,
				partly: true
			}
		]
	},
	{
		key: 'scenes',
		title: 'Scenes and stage',
		intro: 'A scene puts characters’ rigs on a stage: who is there, where the camera looks, what stands behind them.',
		cases: [
			{
				slug: 'new-scene',
				title: 'A new scene',
				summary: 'A scene with its cast, saved as a draft and published in the creator',
				who: ['performer'],
				where: ['editor', 'creator'],
				often: 2,
				steps: [
					{
						text: 'In the “Show” workspace open the “Library” panel and press “new scene”.'
					},
					{
						text: 'Type an “id” – the scene’s address, scene:<id> – and a “name”. From the id on, the scene saves itself as a draft.'
					},
					{
						text: 'Below the cast, find your toon’s rigs and press one to add it to the cast (press again to take it off).'
					},
					{
						text: '“Build” opens the scene on the stage: place the cast and set up its camera there.'
					},
					{
						text: 'Publish it in the creator: “Scenes”, filter by “Toon” and the “Draft” state, then “Publish”. The core checks first that a deck can play it.'
					}
				],
				note: 'Until it is published, a scene shows only in the editor that made it. The rigs come from the server, so sign the editor in first.'
			},
			{
				slug: 'scene-cast',
				title: 'Place the cast',
				summary: 'Move the members and set their values in the scene',
				who: ['performer'],
				where: ['editor'],
				often: 2,
				steps: [
					{ text: 'Open a scene in Build: a double click on it in Show’s “Library”, or “open in Build” on a show cell.' },
					{ text: 'The tree lists the scene and its members. Select one: “MEMBER” shows its holder, filler, spot and rig.' },
					{ text: '“Move” (W) drags a member on the stage, snapping; “Pick” (Q) only selects. “Grid & snap” sets the grid, the step and what it snaps to.' },
					{
						text: 'Under “VALUES IN THE SCENE” drag a knob to set it for this scene; the stage plays it when you let go and the scene saves itself. The “×” beside it goes back to its rig’s value.'
					},
					{ text: '“Open as rig” shows the member’s rig alone with the scene’s values on its knobs. Esc or the scene’s line brings them back into the scene.' }
				],
				note: '“Rotate” (E) and “Scale” (R) switch the mode only; their handles are coming.'
			},
			{
				slug: 'camera-shots',
				title: 'Camera shots and moves',
				summary: 'Lens, shots, moves on beats or seconds, the trolley',
				who: ['performer'],
				where: ['editor'],
				often: 2,
				steps: [
					{ text: 'Open a scene in “Build” and open the “DOP” panel.' },
					{
						text: 'Under “LENS” choose “Perspective” or “Orthographic” and a focal length (17 to 150 mm, or “mm…” for any). The chips frog, cat, kid, adult, giant, giraffe and bird set the eye height.'
					},
					{
						text: 'Frame the stage with the mouse, then press “Add from view” under “SHOTS”. Give the shot an aim: “its point” or a member of the cast.'
					},
					{
						text: 'Under “MOVE” pick a kind (cut, glide, push in, orbit …), the shot it “ends on” and its length in beats, bars or seconds. “Preview” plays it, “Keep as move” keeps it.'
					},
					{
						text: 'A “fly” runs along the “TROLLEY TRACK”: “Add track”, then “Add point” for each stop.'
					},
					{
						text: 'Switch views in the tools pill: “Free view” (0), “Primary camera”, each shot (1–9), “Previous”, “Next”. In a show a cell picks a shot as scene:<id> > shot:<id>.'
					}
				]
			},
			{
				slug: 'parallax',
				title: 'Parallax layers',
				summary: 'Pictures in depth around the 3D, fitted to the camera',
				who: ['artist', 'performer'],
				where: ['editor'],
				often: 1,
				steps: [
					{ text: 'Open a scene in Build and select the scene in the tree.' },
					{
						text: '“Parallax stack” adds seven empty layers: a far matte, three full layers behind the 3D and strips left, right and below in front. “Add layer” adds one more, 4 m behind.'
					},
					{ text: 'Each layer has a handle at its left edge on the stage. Drop a picture from the library on a handle to fill the layer.' },
					{ text: 'Drag a handle up or down to move the layer in depth; the picture is fitted anew to the camera’s view there.' },
					{ text: 'In the “PARALLAX LAYER” inspector “covers” sets full, left, right, top or bottom, and “Remove” takes the layer away.' }
				],
				note: 'A scene takes up to 12 layers.'
			},
			{
				slug: 'module-slots',
				title: 'Module slots',
				summary: 'A motion module on a camera or an item',
				who: ['performer'],
				where: ['editor'],
				often: 1,
				steps: [
					{ text: 'The camera’s slot is in the “MODULES” section of the “DOP” panel; a member’s slot in “MODULES” in its inspector.' },
					{ text: 'An empty slot is dashed: drag a module from the library onto it. It lights up while you carry one over.' },
					{ text: 'Start with dolly on the camera (it pushes in and back on the clock) or bob on an item (it floats).' },
					{ text: 'The filled slot shows its module; “×” takes it off.' }
				],
				note: 'A module needs a build before it fits a slot: “module:… has no descriptor yet – build it first”.'
			},
			{
				slug: 'video-on-stage',
				title: 'Video on stage',
				summary: 'Video nodes, and video as a background or a layer',
				who: ['performer', 'artist'],
				where: ['editor'],
				often: 1,
				partly: true
			},
			{
				slug: 'reader-slots',
				title: 'Slots readers fill',
				summary: 'A place in a scene that a reader or a character fills',
				who: ['performer'],
				where: ['editor'],
				often: 1,
				partly: true
			},
			{
				slug: 'background',
				title: 'A stage background',
				summary: 'A colour or a picture behind a rig or a scene',
				who: ['performer'],
				where: ['editor'],
				often: 1,
				steps: [
					{ text: 'Select the rig (or the scene) in Build: the inspector shows a “background” row.' },
					{
						text: 'Click the swatch for the colour picker: the “PALETTE” (with green and blue screen), your “RECENT” colours, saturation and brightness, the hue.'
					},
					{ text: 'The eyedropper takes the colour under the pointer on the stage; Esc cancels it.' },
					{ text: 'Or click the value and type a colour (#rrggbb) or a picture’s media path.' }
				],
				note: 'A scene’s background wins over its rigs’, and a show’s over both. Empty means the modules’ (or the rigs’) background, else black.'
			}
		]
	},
	{
		key: 'show',
		title: 'Building a show',
		intro: 'A show is a matrix: rows of moments, a cell per outlet, each cell a scene with its values.',
		cases: [
			{
				slug: 'demos',
				title: 'Learn with the demo shows',
				summary: 'Small shows that each show one idea',
				who: ['performer'],
				where: ['editor'],
				often: 1,
				steps: [
					{ text: 'In “Show” open the “Sheet” panel and its “Demos” tab.' },
					{ text: 'A press opens a demo in place of the show. Its first row says what to try.' },
					{ text: '“far-nodes” shows far nodes, “new-scene” making a scene, “values-and-scenes” scenes on cells, “vote-and-question” a vote.' },
					{ text: 'Follow the first row’s “try:” steps, then open the next demo.' }
				],
				note: 'A demo replaces the open show – save yours first. From the command line: --demo NAME.'
			},
			{
				slug: 'show-matrix',
				title: 'Lay out a show',
				summary: 'Rows, outlets and cells; a scene in each cell',
				who: ['performer'],
				where: ['editor'],
				often: 2,
				steps: [
					{
						text: 'In “Show” the “Rows” panel is the show: “new row” adds a moment, each column is an outlet (a deck or a screen).'
					},
					{
						text: 'Drag a published scene from the “Library” onto a cell. A double click opens the scene in Build instead.'
					},
					{
						text: 'Select the cell: “piece” names its scene and “shot” its camera shot. Under “VALUES” set the cell’s knobs; “×” clears one.'
					},
					{
						text: '“open in Build” puts the cell’s scene on the stage. Turn the knobs there, then “values → cell” writes them back and “back to Show” returns.'
					},
					{
						text: 'The show saves itself (Ctrl+S at once). The “Problems” sheet shows what a deck would not play.'
					}
				],
				note: '“Load” in the inspector opens another show.'
			},
			{
				slug: 'row-conditions',
				title: 'What fires a row',
				summary: 'Conditions and addresses that start a row',
				who: ['performer'],
				where: ['editor'],
				often: 2,
				partly: true
			},
			{
				slug: 'node-graph',
				title: 'A show as a node graph',
				summary: 'The same show drawn as nodes and wires',
				who: ['performer'],
				where: ['editor'],
				often: 1,
				steps: [
					{ text: 'In “Show” switch the tools to “2D” and open the “Nodes” panel.' },
					{ text: 'It draws the row: what comes from far, the cells and what the row sends, with typed pins and wires.' },
					{ text: '“LR” or “TB” lays it out left to right or top to bottom; the editor remembers your choice.' },
					{ text: 'The wheel zooms, a middle drag (or Space + drag) pans, “Fit” or F shows it all.' }
				],
				note: 'The Nodes panel hides while a sheet is open.'
			}
		]
	},
	{
		key: 'live',
		title: 'Playing live',
		intro: 'From the editor to the stage: send the show to the decks, run it, and let readers in.',
		cases: [
			{
				slug: 'flash',
				title: 'Send the show to the decks',
				summary: 'Flash the show onto the decks you choose',
				who: ['performer'],
				where: ['editor'],
				often: 3,
				steps: [
					{
						text: 'Open the show in “Show” and press “Flash” in the “Inspector”.'
					},
					{
						text: 'Choose the decks: “all”, “none”, “not on this version”, “ready”, a kind of device or “seat n”. Or type a filter, like where device:browser.'
					},
					{
						text: 'Choose “A suggestion” (each deck has 10 s to say no) or “Forced”, and whether the decks are “Locked”.'
					},
					{
						text: 'Press “Flash n …”, read the question, then “Flash” (or “Force flash”).'
					},
					{
						text: 'A deck shows “Flash · suggestion” with “Accept” and “Reject”. After a forced flash it keeps its own version, so it can go “Back to my version”.'
					}
				],
				note: 'Decks that join later get the flash too. A deck can ask “Please don’t flash”.'
			},
			{
				slug: 'perform',
				title: 'Run the show',
				summary: 'The cues matrix, Next and GO, the log',
				who: ['performer'],
				where: ['editor'],
				often: 3,
				steps: [
					{
						text: 'Switch to “Perform”: “Cues” is the show’s matrix, “Next” what comes, “Stage now” what each outlet plays, the “Log” what happened.'
					},
					{
						text: 'A row’s ▶ fires the whole row; a cell’s ▶ launches only that cell. The stop at the head of a column stops that column.'
					},
					{
						text: '“Next” shows what comes: “GO” plays it, “Drop” skips a suggestion.'
					},
					{
						text: 'Watch “Stage now” and the “Log” (newest first) to see what the decks really play.'
					},
					{
						text: 'Something wrong? “PANIC” (or Shift+Esc) is black and silence on every deck; “RESUME” brings back what the show holds now.'
					}
				],
				note: 'Esc alone leaves the editor – panic is Shift+Esc. Only the “RESUME” button resumes.'
			},
			{
				slug: 'panic',
				title: 'Panic and resume',
				summary: 'Black and silence on every deck, and back',
				who: ['performer'],
				where: ['editor'],
				often: 3,
				steps: [
					{ text: 'The red “PANIC” pill sits in the top bar of every workspace; Shift+Esc does the same from anywhere.' },
					{
						text: 'A panic is black and silence on every deck below this conductor and on the editor’s own stage. The log says “PANIC – every deck below black and silent”.'
					},
					{ text: 'While it holds, the pill reads “RESUME”. Rows still fire in the show, but nothing reaches a stage.' },
					{ text: 'Press “RESUME” to bring every deck back – to the cells the show holds now, not the ones that played before.' }
				],
				note: 'Shift+Esc only panics; only the pill resumes. Plain Esc at the top level quits the editor.'
			},
			{
				slug: 'live-session',
				title: 'A Live session with readers',
				summary: 'A code and a link; readers join; roles and seats',
				who: ['performer'],
				where: ['editor'],
				often: 3,
				partly: true,
				steps: [
					{
						text: 'Start the editor joined to the live session’s relay (--relay and the session’s address). Without it the sheet says “no session”.'
					},
					{
						text: 'In “Perform” open the “Session” sheet: its head shows the session code.'
					},
					{
						text: 'Choose where readers start: “Toon”, “Chapter” and a “Label” (empty = the chapter’s start).'
					},
					{
						text: 'Readers join with the QR code or the link: “Copy link” to send it, “Open here” to try it yourself.'
					},
					{
						text: 'The sheet lists the linked readers and the last story event it heard.'
					},
					{
						text: 'In “Show” the “Who” sheet lists everyone here: “move to …” gives a deck or a phone its role, and “ROLES” → “new role” adds one.'
					}
				],
				note: 'Built in part: today the editor joins a relay by its address; choosing a session in the editor is coming.'
			},
			{
				slug: 'vote',
				title: 'An audience vote',
				summary: 'Tally, hold, close or override a vote',
				who: ['performer'],
				where: ['editor'],
				often: 2,
				partly: true
			},
			{
				slug: 'tempo-sync',
				title: 'Tempo and clock',
				summary: 'The deck’s own clock, tap, MIDI clock in and out, Link',
				who: ['performer'],
				where: ['deck'],
				often: 2
			},
			{
				slug: 'play-scene-on-deck',
				title: 'Play a scene on a deck',
				summary: 'A published scene on a deck, also offline',
				who: ['performer'],
				where: ['deck'],
				often: 2
			},
			{
				slug: 'lights-out',
				title: 'Lights, DMX, OSC and MIDI out',
				summary: 'The bridge and the venue patch',
				who: ['performer'],
				where: ['deck'],
				often: 1,
				partly: true
			}
		]
	},
	{
		key: 'publish',
		title: 'Review and publish',
		intro: 'Everything you make is a draft until it is published. This works the same everywhere in the creator.',
		cases: [
			{
				slug: 'save-preview',
				title: 'Save and preview',
				summary: 'Save a draft and see it as readers will',
				who: ['all'],
				where: ['creator', 'editor'],
				often: 3,
				steps: [
					{ text: '“Save” in the bottom bar stores a draft; a dot on it means there are unsaved changes. Only creators see drafts.' },
					{ text: '“Add a note?” asks what changed – a suggestion is already typed, so just type over it, or press “Save without note”.' },
					{
						text: 'In the chapter editor the round phone button in the top bar opens the “Reader” in the dock; “Screen” switches between “Phone” and “Desktop”.'
					},
					{ text: 'The reader opens at the group and slice you are on and follows you. It shows saved drafts only and reloads after every save.' },
					{ text: 'Unsaved edits wait in this browser: after a reload a bar offers “Restore” or “Discard”.' }
				],
				note: 'In the chapter editor Ctrl+S saves at once, without the note; on the other pages it asks. In the deck editor everything saves itself.'
			},
			{
				slug: 'problems',
				title: 'Fix problems',
				summary: 'The problem list: errors, warnings, a click to jump there',
				who: ['all'],
				where: ['creator'],
				often: 3,
				steps: [
					{ text: '“Problems” in the bottom bar shows its count; press it to open the list. On a phone it is the ⚠ button.' },
					{ text: 'The head counts errors and warnings. Each line says where (group, slice, line, language) and what is wrong.' },
					{ text: 'Click a line to jump to the place and fix it there.' },
					{ text: 'A warning that is fine can be accepted with its ✓ (“Looks fine – accept the warning”). That is an edit like any other, so save it.' },
					{ text: 'Errors can’t be accepted. Publishing with errors asks “Publish anyway?” first.' }
				],
				note: 'Tick “accepted” or “notes” to see the accepted warnings and the quiet notes again.'
			},
			{
				slug: 'publish',
				title: 'Publish',
				summary: 'Publish one file or everything, or submit it for review',
				who: ['all'],
				where: ['creator'],
				often: 2,
				steps: [
					{
						text: '“Save” (or Ctrl+S) stores a draft: only creators see it. The note “Save” asks for says what changed.'
					},
					{
						text: 'The phone icon in the top bar opens the “Reader” in the dock – choose “Phone” or “Desktop”. It shows your saved drafts as readers will see them.'
					},
					{
						text: '“Problems” in the bottom bar lists errors and warnings; “Go to the place” jumps there. A warning that is fine can be accepted, an error can’t.'
					},
					{
						text: '“Publish” lists every file with a draft readers don’t see yet. Untick what should wait, then press “Publish n”.'
					},
					{
						text: 'Does your toon have reviewers? Then the button is “Submit for review”: add a “Note for the reviewers” and press “Submit” – they publish it or send it back.'
					},
					{
						text: '“Publish” on the start page does it for everything at once: every toon and the world (map, characters, settings).'
					}
				],
				note: 'Published means readers see it right away. Every earlier version stays in “History” → “Versions”.'
			},
			{
				slug: 'review',
				title: 'Review others’ work',
				summary: 'Publish a submission or send it back with a note',
				who: ['lead'],
				where: ['creator'],
				often: 2,
				facts: [
					{
						label: 'Who reviews',
						text: 'Set in the admin per account and toon: “Reviews publishing requests” or “Submits for review”. Without a rule, an account publishes itself.'
					}
				],
				steps: [
					{ text: 'When work waits for you, the start page shows “Waiting for you” with its count.' },
					{ text: 'Each submission shows the toon, the files, who sent it, when, and their note. Click its name to open the page and look.' },
					{ text: '“Publish” makes it live as submitted; the author gets a mail.' },
					{ text: 'Or “Send back” with a note to the author: they get it by mail, and the drafts stay as they are.' }
				],
				note: '“Newer drafts exist – not part of this review” means the author changed more since: only the submitted version goes live.'
			},
			{
				slug: 'history',
				title: 'Version history',
				summary: 'Restore an earlier version, version notes, unpublish',
				who: ['all'],
				where: ['creator'],
				often: 2,
				steps: [
					{ text: '“History” in the bottom bar holds “Undo”, “Redo” and “Versions”. Outside a text field Ctrl+Z undoes and Ctrl+Shift+Z redoes.' },
					{ text: '“Versions” lists every saved version: when, by whom, its note, and whether it is “current” or “in review”.' },
					{ text: '“Restore” brings an earlier version back as a new draft. Nothing is published until you publish it.' },
					{ text: 'A published version has “Unpublish”: readers then see nothing until a version is published again.' }
				],
				note: 'Chapters and seasons hold every language in one version, so unpublishing takes them all. Accounts that submit for review see no Publish or Unpublish here.'
			}
		]
	}
];
