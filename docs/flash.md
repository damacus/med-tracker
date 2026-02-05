Skill Module: UI Feedback & Alerting
Objective: Ensure the LLM generates or validates user feedback mechanisms (toasts, banners, inline errors) that reduce cognitive load and respect visual hierarchy.

✅ What to Do (The Rules)
Enforce "Contextual Proximity"

Instruction: Always place error messages immediately adjacent to the element that caused them.

Logic: If a specific input fails, use an Inline Validation. If a form submission fails, use a Card Alert at the top of the form. Only use Global Toasts/Banners for system-wide state changes (e.g., "Wifi Lost").

Match "Visual Weight" to "Severity"

Instruction: Classify the error severity before choosing the UI component.

Logic:

Low (Info/Success): Temporary Toast (disappears automatically).

Medium (Validation): Inline text or local alert box (requires user fix).

High (System Failure): Modal or Sticky Global Banner (blocks interaction).

Deduplicate Signals

Instruction: Check for redundant messaging paths.

Logic: If an inline error exists (<input class="error">), do not also trigger a global notification for the same error. Choose the most specific location.

Actionable Copywriting

Instruction: Error messages must propose a solution.

Logic: Replace state descriptions ("Login Failed") with instructions ("Check your password and try again").

❌ What to Avoid (The Anti-Patterns)
The "Double-Bank" (Redundancy)

Constraint: Never output a layout that renders the same error string in two DOM locations simultaneously (e.g., a top banner AND a form alert).

The "Global-for-Local" Fallacy

Constraint: Do not use full-width, top-of-viewport bars for local interactions (like login prompts or field validation). This trains "banner blindness."

Vague Headers

Constraint: Avoid generic titles like "Error" or "Warning." The visual style (Red/Yellow color) already communicates the category; the text should communicate the content.

Implementation: Example System Prompt
If you are feeding this into a custom GPT or an agent workflow, you can paste this directly into its instructions:

[UI Feedback Protocol] When generating or critiquing UI code for alerts:

Analyze Scope: Is this error Global (system-wide) or Local (task-specific)?

Apply Layout:

IF Local: Inject alert inside the container/card, above the primary inputs.

IF Global: Use a fixed-position toast or banner.

Review: Reject any design that separates the feedback from the action by more than 200px or duplicates the message.
