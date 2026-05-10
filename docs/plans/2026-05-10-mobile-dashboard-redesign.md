# Mobile Dashboard Redesign

## Goal

The mobile dashboard should behave like a daily medicine command surface, not an analytics page. The first useful read should answer three questions:

- What medicines are due today?
- What needs action now?
- Which button do I tap: Take or Give?

## Visual Direction

Use a clinically polished app feel with clear colour semantics:

- Teal and green remain the brand and successful-completion colours.
- Blue, purple, amber, teal, and rose are used as medicine/status accents so the list is scannable.
- Surfaces stay light, spacious, and softly layered, but cards should have deliberate coloured borders, icon wells, and action buttons.
- Medicine cards are the dominant content. Summary information and filters support the list instead of competing with it.

## Required Mobile Structure

1. Top app bar with menu, MedTracker brand, and notifications only.
2. Page title/date area for today's medicines.
3. Compact progress strip showing completed doses, next due time, and a small progress indicator.
4. Filter chips for All, Needs action, Upcoming, and Taken.
5. Colour-coded medicine cards with large icon wells, compact metadata, status chips, and a Take/Give action only when needed.
6. Quiet bottom navigation.

## Filter Behaviour

Filters are important because families often need to switch between the full day and the immediate work queue.

- All: every dose for today.
- Needs action: doses that still expose a Take/Give action.
- Upcoming: scheduled future work.
- Taken: completed doses.

Filters should be URL-addressable so they survive refresh and can be tested without JavaScript.

## Medicine Card Rules

Each card must show:

- Medicine name.
- Person name, time, and location in compact metadata.
- Status chip.
- Take/Give button only when the dose can still be recorded.

The status colour should be obvious before reading the text:

- Taken: green.
- Needs action / primary upcoming: blue or purple.
- Child/care-giver action accents may use amber or teal when useful.
- Blocked states use warning or error colours without showing a primary action.

## Implementation Notes

Implement one stronger default first: a filtered, colour-coded schedule stack inspired by the reference images. Keep a profile experiment option available as a future follow-up if we decide to compare multiple dashboard structures such as time-rail, family-grouped, or dense action-stack layouts.
