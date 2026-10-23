# Person editing reference

The reference uses source `aa52085e08dd56da234ef4b014960a20cd6bbb0d`
(dependent creation PR #2896), with the new browser journey only. It signs in
through the normal application as the synthetic adult in the persistence-fixture
household, then edits that person's name, email and birth date. The fixture uses
English, light appearance and the existing account timezone.

Desktop is 1440 by 1000; mobile is 390 by 844. Capture waits for hydrated navigation
and disables animations. Both are full-page images. The baseline journey saved
the form successfully and checked the persisted name, normalised email and date.

The typed candidate passed the same journey and database assertions. Both final
images are byte-for-byte identical to their baseline images. The images were
visually inspected: fields, labels, layout, focus, navigation and actions retain
the reference presentation. The mobile fixed navigation appears at its viewport
position within the full-page image; the save action remains reachable and the
browser test submits it successfully.
