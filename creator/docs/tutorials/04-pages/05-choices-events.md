---
slug: choices-events
title: Choices and events
summary: Visual-novel events, choices, conditions and jumps
who: writer
where: creator
often: 2
---

## Steps

1. Select a slice and open the “Events” tab, or draw an area with the “Event” tool for a tap event. For a whole group: its “Visual novel” section → “+ VN script”.
2. Add events with the buttons: “+ showMessage” for dialog, “+ addChoice” and “+ showChoices” for choices, “+ jumpTo” to go elsewhere.
3. A choice has a “Text” and what happens “When picked”: set a variable, give an item, jump to a group, continue at a point.
4. Points are “+ label” events; “+ divert” and choices continue there.
5. “+ Only if” puts a condition on any event: “Simple” (variable, comparison, value) or an “Expression” like count("e…") < 3.
6. Watch the chips: “may get stuck”, “unreachable” or “endless loop” say where readers could not go on.

## Note

A speaker who is not on stage is skipped – the fix button “+ joinScene before” brings them in.
