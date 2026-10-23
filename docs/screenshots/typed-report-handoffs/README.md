# Typed report handoff browser evidence

The `reference/` images were captured from unchanged application source at
`d0469001f77f50b6854b0e5bd1ffeda013c2d3b5`, before the report refactor.
The adjacent images show the final typed candidate. Both runs used the same
synthetic persistence fixture and waited for `body[data-navigation-ready]`
before capturing the reports page. The desktop viewport was 1440 × 1000;
mobile viewports were 390 × 844 and 320 × 844.

The unchanged-source and candidate `shared_layout` browser runs each passed
7/7 journeys. All three reports screenshot pairs are byte-identical. The
reports page, navigation, selection controls and responsive layout render the
same after report data moves through typed rows and view models.

| Viewport | Unchanged-source reference | Typed candidate | SHA-256 of both images |
| --- | --- | --- | --- |
| Desktop | [reference](reference/reports-desktop.png) | [candidate](reports-desktop.png) | `f7e6920d2bf0a432b589dc26d92ea9b42c728061935465740934b65a190edd9e` |
| 390px | [reference](reference/reports-390.png) | [candidate](reports-390.png) | `856ed96acae19f5e599be89337b1a2c8319731918f6cc6970cae463c52b38cd8` |
| 320px | [reference](reference/reports-320.png) | [candidate](reports-320.png) | `07479b110146636635a12d1d822eebe4441b6c36e33caf2367bb93df009b4bef` |

The final browser Task output is saved at
`/private/tmp/layer5-reports-browser.log`. API acceptance separately checks
report JSON, PDF generation, font embedding, locale, current authority and
download audit effects.
