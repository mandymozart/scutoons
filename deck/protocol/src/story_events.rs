// GENERATED from server/schema/content/chapter.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
//! The story events (#517): the type of every event a chapter's visual novel takes (`VNEvent`), what a scene's listener
//! may name (deck-space scene_document.rs `Listener`).

/// every VNEvent type, in the chapter schema's order
pub const STORY_EVENT_TYPES: [&str; 18] = ["showMessage", "showChoices", "addChoice", "joinScene", "leaveScene", "hideMessage", "showTextBox", "fx", "unlockLocation", "migrate", "setAvatarLocation", "jumpTo", "label", "divert", "setVariable", "giveItem", "takeItem", "askText"];
