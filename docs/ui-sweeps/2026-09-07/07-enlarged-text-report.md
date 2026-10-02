# 07 enlarged text verification

The focused browser regression was first run red before the enlarged-text production changes,
using the prepared local Docker overlay. Canary was not used.

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/enlarged_text_spec.rb
```

The red run recorded **2 examples, 2 failures**. At 320px with the document root at 32px, the
search control had a zero-sized rectangle, the brand and mobile rail labels exceeded their
allocated bounds, the warning text overlapped its dismiss area, page overflow reached 129px, and
the four profile tabs were laid out as one unreadable row. The initial profile-tab geometry was
also measured during a `transition-all` animation, so later verification waits for the computed
24px tab font before measuring settled geometry.

The bounded fix added `min-w-0` and truncation to the mobile shell, natural-height wrapping for
rail labels, a mobile warning layout with the dismiss control above the description, and narrow
profile layout rules. The profile tab trigger now gives its label an explicit full-width wrapping
span so long labels such as Notifications and Advanced cannot use an anonymous flex item's
automatic minimum width. Personal information, section summaries, and the profile hero also wrap
their measured long values within their cards.

The permanent focused regression now checks menu/search/brand bounds, every rail label, warning
text ranges against the dismiss control at root font sizes 16px and 32px, desktop warning inset
and text-range separation, personal-information and summary content, hero content, page overflow,
and profile-tab rows. At 320px and root font size 32px it requires two rows of two tabs with each
label contained in no more than two lines. It revisits the profile at 1280px and root font size
16px before requiring one row of four tabs.

The final focused run was green:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/enlarged_text_spec.rb
2 examples, 0 failures
```

The permanent spec waits for the settled 24px tab font using animation-frame polling; it does not
assert against the intermediate 12px transition state. The geometry helpers are stored in scoped
JavaScript constants, and `rubocop --cache false` reported no offenses for the touched focused
spec and related UI audit files.

Final screenshots were captured by a temporary one-example browser spec using the same overlay,
then inspected and the temporary spec removed:

- [`enlarged-text-320-fixed.png`](../../screenshots/ui-sweep/enlarged-text-320-fixed.png): shell and warning at 320px with root font size 32px.
- [`profile-tabs-320-enlarged-fixed.png`](../../screenshots/ui-sweep/profile-tabs-320-enlarged-fixed.png): notice dismissed and hidden, with two rows of two contained profile labels at 320px and root font size 32px.

The capture completed with **1 example, 0 failures**. The final images replace the rejected
intermediate profile capture; no screenshot-only example remains in the permanent suite.
