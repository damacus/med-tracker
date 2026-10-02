# Issue #2125: routine dashboard headline counts

Dashboard headline metrics now read routine rows only. As-needed availability,
cooldown, disclosure content, and take controls remain on their existing path.

The presenter red run (`spec/presenters/dashboard_presenter_spec.rb`) had 32
examples and three failures: a routine-now case counted an extra as-needed row
(expected 1, got 2), an as-needed-only case returned `Now` instead of `None
today`, and a mixed routine/cooldown case returned `14:15` instead of the
routine `16:30`. Coverage now includes routine-only, as-needed-only available
and cooldown rows, mixed future routine plus available or cooldown as-needed
work, and terminal rows.

The presenter green run passed with 33 examples and 0 failures. The browser
regression keeps an as-needed medication available after the routine dose is
taken, verifies the routine headline reaches `None today`, `0`, and `0`, and
opens the as-needed disclosure to confirm its labelled take control remains
available without recording an as-needed take. The focused dashboard system
run passed with 6 examples and 0 failures.

Local screenshots were captured from that browser scenario with the disclosure
expanded:

- [desktop dashboard](../../screenshots/issue-2125/dashboard-routine-complete-desktop.png)
- [mobile dashboard](../../screenshots/issue-2125/dashboard-routine-complete-mobile.png)

The captures show `None today`, zero Due Now and Tasks Left, the completed
routine section, and the expanded `AS NEEDED` Ibuprofen control. Each viewport
was resized and reloaded before capture so its responsive media state settled;
the temporary capture code was removed from the permanent regression.
