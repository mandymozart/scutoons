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
						text: '“Save” (or Ctrl+S) keeps it all as a draft, with a short note if you like.'
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
				often: 2
			},
			{
				slug: 'kanon',
				title: 'Write the Kanon',
				summary: 'The story text behind a toon, a character or a place',
				who: ['writer'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'story-flow',
				title: 'Check the story flow',
				summary: 'Where readers go, and where they get stuck',
				who: ['writer', 'lead'],
				where: ['creator'],
				often: 2
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
				often: 2
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
				often: 2
			},
			{
				slug: 'choices-events',
				title: 'Choices and events',
				summary: 'Visual-novel events, choices, conditions and jumps',
				who: ['writer'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'variables',
				title: 'Variables in text',
				summary: 'The reader’s name and other values inside a text',
				who: ['writer'],
				where: ['creator'],
				often: 2
			},
			{
				slug: 'chapter-sound',
				title: 'Music and samples in a chapter',
				summary: 'Music volumes per slice, samples on scroll or on a click',
				who: ['sound'],
				where: ['creator'],
				often: 2
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
				often: 1
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
				often: 2
			},
			{
				slug: 'mixer',
				title: 'Mix a rig',
				summary: 'Faders, gains inside a track, pan, mute and peaks',
				who: ['sound'],
				where: ['editor'],
				often: 2
			},
			{
				slug: 'module-params',
				title: 'A module’s parameters by page',
				summary: 'Areas and pages, eight knobs a page, the Analog Four way',
				who: ['sound'],
				where: ['editor'],
				often: 3
			},
			{
				slug: 'own-module',
				title: 'Write your own module',
				summary: 'Write, build and hear a module; save it as a draft',
				who: ['sound'],
				where: ['editor'],
				often: 1
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
				often: 2
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
				often: 1
			},
			{
				slug: 'module-slots',
				title: 'Module slots',
				summary: 'A motion module on a camera or an item',
				who: ['performer'],
				where: ['editor'],
				often: 1
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
				often: 1
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
				often: 1
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
				often: 1
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
				often: 3
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
				often: 3
			},
			{
				slug: 'problems',
				title: 'Fix problems',
				summary: 'The problem list: errors, warnings, a click to jump there',
				who: ['all'],
				where: ['creator'],
				often: 3
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
						text: '“Save” (or Ctrl+S) stores a draft: only creators see it. The note you add says what changed.'
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
				often: 2
			},
			{
				slug: 'history',
				title: 'Version history',
				summary: 'Restore an earlier version, version notes, unpublish',
				who: ['all'],
				where: ['creator'],
				often: 2
			}
		]
	}
];
