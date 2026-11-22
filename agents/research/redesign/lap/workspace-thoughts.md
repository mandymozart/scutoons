Status: thought aloud by the owner 2026-11-22, answered (63, 64) and released for building 2026-11-23 ("i want to see the concepts all implemented. including the crafts") – the plan is ../LAP.md, tickets #820–#830.

# Stage, panels and workspaces – thoughts

1. **No push-aside.** "why should it push anything asside." Stage content uses the whole window; panels float over it. (Today a page on the stage gets padding for the panel columns, even folded ones – from the stage.html draft, built in #765, never an owner decision.)
2. **The "portal" leaves pills everywhere.** "in the end the portal kind of conept is also a bit shitty, since i end up with panel pills all over the place." A tab on the stage leaves its panel head behind, and the stage adds its own pill.
3. **A tab dropped on the stage should be able to become its own panel.** "with our current drag tab to stage flow i can not drag a view into its own panel anymore. it would automatically take a slot." (The 2026-11-20 universal-tabs decision, after OSCT: a tab dragged out onto the stage = its own floating panel.)
4. **The Views menu lists views, not panels.** "the views menu should show each view as it self. not grouped into panels. that is super weird." → #809 (rewritten).
5. **No Tree in the character workspace.** "why is there a tree? our library has the same function. i just open a character from there."
6. **Characters and scenes in relation to assets.** "so our question is how do we handle a character in relation to assets. same goes with scenes."
7. **Most workspaces are the same.** "overall i am thinking a lot more workspaces are basically the same, a stage main view for any asset somebody opens, and additional panels to change what we see there."

8. **Leaving the stage through the Views menu.** "also questions how do we have something leave the stage? i think just with the view menu?" (Then no pill on the stage, no shrink button on a panel head: the Views menu is the one place that says where each view is.)

9. **Saving workspace layouts.** "and how do we save workspace layouts." (Today: each workspace's preset is fixed in code (creator/src/lib/shell/presets.ts); what you move is kept per browser in localStorage (`sc-creator-tabs`); Views › "Back to the preset" resets. No named layouts, nothing per account, nothing shared.)
10. **General enough → the perfect workspaces.** "because this way we will eventually have the perfect workspaces if our general concept is general enough" – saved layouts are how the general model becomes specific workspaces.
    Related, already decided for the deck editor (decisions/deck.md 2026-11-18 [workspaces]): the Asset view is "an isolated stage for one entity" (a character with its rig, an item, a module); a panel is defined once and any view can show it.

11. **A layout for every workspace – think Blender.** "a layout for every workspace. think blender for a second." (Blender: workspace tabs along the top – Layout, Modeling, Sculpting, Shading, Animation …; each is a saved layout of areas, any area can switch to any editor; the open file / object stays the same while you switch; workspaces can be added, duplicated, renamed, and the defaults come from the startup file.)

12. **Role-specific layouts as the end goal.** "eventually the goal is to have role specific layouts in the end. workspaces that show what a specific role needs." (Today's account roles are access rights only: member, admin, creator, developer, live – server/src/Domain/Roles/Roles.php; no craft roles like writer, illustrator, 3D artist, sound, director.)

13. **Users customize their layouts.** "and the user can customize their layouts" – on top of the role's defaults (agent's reading: own copy per person, saved with the account; "back to the role's default" as the reset).

14. **Layouts are saved in the database.** "yes in database"
15. **Saving a workspace; reset = its last saved state.** "workspace saving makes sense. so reset to the last saved state of that workspace." (Not back to a code preset: you save a workspace, you move things, reset returns to what you saved.)
16. **Workspaces follow workflows.** "workspaces kind of follow workflows"
17. **The dashboard's workspace overview = the user's collection of workflows.** "so the workspace overview on the dashboard is basically a workflow collection of a user …"
18. **Presets per role – but not called "role".** "presets we are now developing for roles. maybe role is confusing with other terminologies. but responsibilities or personas (which is irritating with the character personas) but used in user stories." First list, "from the top of my head": show runner · illustrator · video editor · writer · translator · animator · sound designer · developer. (Agent's note: "role" is taken by the account rights member / creator / admin / developer / live; "persona" by the characters. Candidates: craft · hat · desk · job.)

19. **Craft → workflows → workspaces; the dashboard shows them; signed up for or assigned.** "i mean each craft has workflows hence needs workspaces that make sense to them. and the dashboard should show those. a user can create workspaces for each craft they signed up for. or get assigned to." (The owner uses "craft" here.)
    Agent's reading: a user has crafts (chosen at sign-up or assigned by a lead); each craft brings its default workspaces (one per workflow); the user adds or changes their own within a craft; the dashboard groups the user's workspaces by craft.

20. **Assigning crafts is an admin feature for now.** "its an admin feature at this point. so in the invite and in the user detail page admins can assign crafts. perhaps also an onboarding for new users" – the admin picks crafts in the invite and on the user's detail page; maybe an onboarding for new users (agent's reading: a new user's first run says which crafts they have and opens their workspaces, perhaps lets them pick more).

21. **No explaining in the onboarding.** "dont do the explain... we have to design intuitively ... worst case we got docs." Onboarding = show the user's crafts and open their workspaces; no tour, no tips. The design must speak for itself; docs are the fallback.

22. **The creator slowly dilutes into the deck editor.** "assume we are also slowly diluting the creator into the deck editor. so eventually more features gonna transition into the deck since it is overall more performant and we can bring parts back to the web still if we need to through wasm" · then: "of course stuff that is deck related... of course only" – only deck-related features move. Agent's reading: crafts, workspaces, saved layouts and the stage / panel / view rules are defined once in the core (Rust), so the deck editor (deck/kit) and the web (via wasm) read the same model; nothing of it is built creator-only.

23. **An empty slot: pick, upload or generate – in the Library; not this iteration.** "empty mood does not necessarily open the generate workspace. it can be filled by an asset from the library, so the pick functionality or you can upload or you can generate. i assume all this happens in the library, but this is not part of this iteration. store this info where ever we worked on asset slots" → commented on #752; to go into decisions/creator.md and AI-HOOKS.md §0 with the next docs commit.

24. **One launcher instead of many search fields.** "here is a bunch of search to open. perhaps we can simplify any type to open, like the mac quick launch ... cmd or alt space bar. this would really simplify opening stuff and jumping through workspaces as well as jumping to assets" – one key (Alt+Space / Cmd+Space) opens one field for items, workspaces and assets.
25. **Work a bit like macOS and Arch Linux** (Spotlight / a keyboard launcher like rofi; tiling-like keyboard moves). "can we work a little bit like mac os and arch linux?"
26. **Gestures as on Android: drag a panel off the screen to close it** – "and android? there is the thing where you can drag a panel and drag it out of the screen to close. like we got the drag to a location on the screen to take over the stage." (Agent's note, 2026-11-22: the deck has no swipe gestures yet – only the editor's press-to-carry (deck/kit/src/editor/carry.rs) and a touch grip on the live panel.)

27. **Snap like Windows: edges split, corners take a quarter; only a fling closes.** "dragging a panel to the left or right side without the fling, it would do the split. i love the windows four corner concept also, where you can also take a quarter of the stage." – slow drag to the left / right edge = that half of the stage; to a corner = that quarter; top edge = the whole stage; a fast fling past an edge = close.

28. **The landing page: one flat list of workspaces, most used first** (superseded the first ask "group workspaces into crafts on the landing page" – owner: "Flat everywhere"). "just perhaps order by most used". Crafts only decide which workspaces someone starts with; nowhere are workspaces grouped by craft.
29. **Drop zones reach the screen's edges, over the bar and status bar.** "dropzones should extend to the edge of the screen, even when the top bar and footer are underneath them. because for this action it doesn't matter"

30. **The bar does not group workspaces by craft.** "also top bar dont group workspaces by craft." · "dont group the workspaces by craft!!!!!" – one flat row of workspaces (most used), no craft label.

31. **No "Wanted pictures" workspace.** "wanted pictures is a stupid workspace." (Agent's reading: what's missing is a filter in the Library, not a workflow of its own.)

32. **The menu sits in different corners in the deck and the creator.** "also i notice in our deck the menu is top right and in the creator it is top left. that is weird" (Agent's check, 2026-11-22: the deck editor's code puts its hamburger top left (deck/editor/src/bar.rs) and lists the panels inside it, while the creator has the hamburger left and Views / History / Publish right – one place for both needed; which deck screen the owner means is to confirm.)

33. **Generate is a canvas, not a picture tool.** "generate can produce more than pictures.... its a canvas.... as described in their definition." – the spore canvas (AI-HOOKS.md §0): the output spore defines what it makes (picture, 3D model, sound, video, text …).

34. **Build info into the menu; one bar for deck and creator.** "can we move the build info into the menu, so we gain space. compare top bars deck and creator. to unify" – the version (and the account, the load meters) go into the main menu; both apps get the same bar. (Supersedes deck.md 2026-11-08 [platform] "every build shows its version top right, always" – its reason, seeing that an update arrived, moves to the menu line and a one-time status-bar note. Draft: crafts.html screen 0.)

35. **Publish hides when nothing needs publishing.** "the publish button can be hidden when nothing needs publishing right?" – agent's reading: its scope is the open item (on Home: everything not live, #793); hidden when that scope has nothing unpublished; it sits at the row's end so nothing shifts when it appears.

36. **The menu far right; History a view; Views in the menu.** "i propose menu far right, than publish and user. history goes to views and views goes to menu" – right end of the bar: … Publish · account · menu (far right); History is no longer a bar menu but a view; the Views list lives in the main menu (with Save workspace, Settings, Docs, the build).

37. **Icons only in the bar; search is a key first.** "search can go there. all those top ones just have icons. no text. search is primarily cmd space or alt space, and on the pad controller the Y" – the bar's right end: undo · redo · Publish · account · menu, icons without text.
38. **The menu button is the dashboard, app-switcher style; no Home button.** "dashboard is main menu button. dashboard is app switcher style no need for the top left home button." – open items as cards (fling to close), workspaces most used first, at the side the account, the front item's Views, Not live yet, Settings, Docs, the build.
39. **Search also sits in the menu, for those who don't know the keys yet.** "we can also move that one into the menu" · "just in case someone doesnt know yet how to use those other ui concepts" (agent's reading: "that one" = search).
40. **No drag handles on panels.** "panel drag handles can go. whole top is drag handle except when dragging the name, than it becomes the tab you move."
41. **The active tab's filled pill is too strong.** "i think those pills we use for those pads are a bit extreme when active. what alternatives do we got?" – drawn: A pressed fill (today) · B quiet fill (selected) · C accent line · D text only · E thin outline; agent's pick B.

42. **Open items show inside their workspace, with a preview.** "show in their workspace with a preview." – no separate row of open items: each workspace card previews what is open in it.
43. **The dashboard's side column was unclear.** "whats that thing on the right?" – removed: the account, Settings, Docs and the build become one quiet line at the bottom; Not live yet is the Publish card; the front workspace card carries its Views.
44. **Tab moves between the dashboard's cards.** "tab moves between those suggestions." – Alt-Tab style: Tab / Shift Tab (the pad's stick) move, Enter opens, Esc or the menu button goes back.

45. **Panel heads without their own background colour.** "i dont want the header to be this separated color background thingy."
46. **The active tab: a thin outline, the others slightly less bright.** "a thin outline and the others slight less bright looks good to me at this point." (Style E + D; also for the bar's item and workspace tabs.)
47. **The item tabs scroll and keep their place.** "when opened asset list should scroll like in any other software when there is too many open and they should not jump to the first position when clicked." – a horizontal strip with faded ends and ‹ ›; a clicked tab stays where it is; order changes only by dragging; new tabs open right of the front one.
48. **How item tabs and workspace tabs relate.** "lets review all this and include a concept for how asset and workspace in top relate." – drafted (crafts.html "Item + workspace"): an open item remembers its workspace; clicking an item tab brings it back in its workspace; clicking a workspace keeps the item if it fits; a workspace that doesn't fit is dimmed and opens its own last item; something new opens in the front workspace if it fits, else its kind's most used one.

49. **At the first login a user is asked what their crafts are.** "at first login a user gets ask what their crafts are. that what i meant, before. btw. maybe we design this view too." – drafted: "Welcome, Mira · What do you do?", the eight crafts as tiles, the invite's ones ticked ("from your invite"), Continue → the dashboard; changeable in Settings. (Adds to point 20: admins still assign in the invite / user page.)
50. **The dashboard shows last opened, and what needs review or isn't published.** "the dashboard show last opened and needs review or unpublished."

51. **Redraw every screen as one set; drop what was rejected; show the overlays and open menus.** "nice now lets do all the screen again. remove those that we rejected. including the overlays and open menus"
52. **The canvases and the deck's workspaces in the same model:** "also do the canvases and the workspaces on the deck · xr · rigging, etc. scene etc."
53. **How the deck opens before anything was done.** "also how the deck would open before anything was done."
54. **A drive logic, similar.** "i assume we have to have a drive logic similar." (Agent's reading, to confirm: the session state – open items, their workspaces, panels, stage places, scroll and selection – is kept with the account like a drive, so it follows the person between machines and apps.)
55. **Reopening resumes exactly, like a hibernated computer.** "important when re opening, the same view as last time opens, like a hibernated computer."

56. **Swipe up from the bottom: the user menu, offering to switch the signed-in user.** "swipe up from the bottom shows the user menu, and asks if you want to switch the user logged in."

57. **Day mode: the background is white.** "once we start this redesign lap, we also should fix the background in the day mode... needs to be white. =) fyi" (Today shared/tokens/editor.css has `--stage: #000000` in day too, from the 2026-11-21 dogma "the ground is black (the stage)" – this changes it for day mode.)

58. **A panel collapses by double-clicking its head; no minimize button.** "and one other rule for the tab... collapse just by double click on the header, no need for a minify button"

59. **No buttons for putting a panel in a place.** "and the buttons to go into a certain place go away. we got most of them covered by dragging" – the panel head's "take the stage" / expand buttons go; dragging (snap) and the Views list cover it.
60. **Full screen over everything** – asked "no rule for overlay fullscreen yet. give suggestions"; answered: *"A fourth state in Views: hidden, in a panel, on the stage, and full screen. 1. yes 2. yes 3. no 4. yes 5. yes"* – so: (1) a key toggles it for the view under the pointer (Ctrl Space, as Blender; Esc leaves); (2) drag past the top edge and hold ≈ 0.5 s: the whole-stage preview turns into full screen; (3) no fling up; (4) Views gets a 4th state "full screen"; (5) the pad's shoulder button and a two-finger spread on touch toggle it.
61. **Animations for the transitions and for hot zones activating.** "and we will need some animations for these transitions and when things are getting activated like the hotzones"

62. **The welcome screen also shows unpublished items.** "the welcome screen should also show unpublished items. please replace that screen." – crafts-v2 screen 9: "Not published · 18 – in the toons you work on" above the empty workspaces.

63. **Answered (owner, 2026-11-22, before the lap):**
    - A character is **an item with assets** (a document like a chapter or a scene; the Library finds and opens it; its pictures, model, voice are assets attached to it).
    - **Any view** can take the stage (the four states in Views stand).
    - The split is simple: the stage's layouts are *"1/1 or 1/2/2 or 4/4/4/4 or 2/2/1"* – one whole · a half + two quarters · four quarters · two quarters + a half; placed by dragging (points 27, 60).
    - Point 32 is moot: the menu is far right everywhere.
64. **Panels never snap; they float on top at all times and ellipsize cleanly.** "the snap layout the first recommendation makes sense, but its confusing since i dont want panels to snap, also they should watch how they do ellipsises when becoming too small, so there is no text float mess. and they float on top of the stage at all times. today the push shit around, that is wrong as our drafts show." – dragging a panel to an edge or corner puts its *view* on the stage; the panel itself never docks or snaps. A panel that gets small truncates its tabs and labels with an ellipsis (no wrapping, no overflow). Nothing pushes the stage aside (point 1).

## Open questions (answered 2026-11-22 – see 63) (agent's, for the owner)
- Is a character an asset (a Library entry like a picture or a model, that opens on the stage), or an item that *has* assets?
- Is "workspace" then just: what is on the stage (one opened thing) + a preset of panels for its kind?
- What is the stage for – only the opened thing, or any view?
- How does something take the stage, and where does the previous one go?
- Split: two things on the stage, or a second view always a floating panel?
- Folded panels: in place, or one row (the status bar, as the deck editor)?
