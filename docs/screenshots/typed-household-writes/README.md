# Typed household write browser evidence

These captures use synthetic fixtures and the browser harness's `en-GB` locale.
Each screenshot follows a visible `body[data-navigation-ready]` guard. Desktop
viewports are 1440 × 1000; mobile viewports are 390 × 844. The unchanged-source
baseline used parent commit `6bc67a0fa6a330d144332b44d94593867331bb78`:
its full browser run passed 42 journeys, and its focused storage-location run
passed two journeys after adding the capture guard. The final full browser run
used the typed household write candidate and passed 42 journeys. Each full run
had one skipped test.

| View | Baseline | Final | Comparison |
| --- | --- | --- | --- |
| Location deletion, desktop | [image](baseline/storage-location-deletion-confirmation-desktop.png) | [image](final/storage-location-deletion-confirmation-desktop.png) | Byte-identical |
| Location deletion, mobile | [image](baseline/storage-location-deletion-confirmation-mobile.png) | [image](final/storage-location-deletion-confirmation-mobile.png) | Byte-identical |
| Invitation management, desktop | [image](baseline/invitation-management-management-1440.png) | [image](final/invitation-management-management-1440.png) | Same layout and content; synthetic fixture timestamps reflect each run's clock |
| Invitation management, mobile | [image](baseline/invitation-management-management-390.png) | [image](final/invitation-management-management-390.png) | Same layout and content; synthetic fixture timestamps reflect each run's clock |
| Household settings, desktop | [image](baseline/shared-layout-admin-settings-1440.png) | [image](final/shared-layout-admin-settings-1440.png) | Byte-identical |
| Household settings, mobile | [image](baseline/shared-layout-admin-settings-390.png) | [image](final/shared-layout-admin-settings-390.png) | Byte-identical |
| Household members, desktop | [image](baseline/shared-layout-admin-members-1440.png) | [image](final/shared-layout-admin-members-1440.png) | Byte-identical |
| Household members, mobile | [image](baseline/shared-layout-admin-members-390.png) | [image](final/shared-layout-admin-members-390.png) | Byte-identical |

The invitation screenshots were visually compared at both sizes. Only the
displayed created, expiry and delivery-attempt times differ. The browser suite
also exercises the forms, validation and keyboard actions beyond these captures.
