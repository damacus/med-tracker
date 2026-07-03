# PWA Web Push Test Plan

## Browsers

- Chrome or Edge desktop
- Chrome or Edge Android
- Safari macOS where Web Push is available
- iOS or iPadOS from an installed Home Screen PWA where Web Push is available

## Setup

1. Sign in to a test household with at least one active person, one active schedule, and one stocked medication with a reorder threshold.
2. Open profile notification settings.
3. Confirm the browser reports push support, then enable notifications.
4. Send a test notification and confirm the notification opens or focuses Med Tracker.
5. Disable notifications for the browser and confirm the current browser subscription is removed.
6. Re-enable notifications before continuing.

## Dose-Due Reminder

1. Set a schedule time a few minutes in the future.
2. Confirm dose-due notifications are enabled.
3. Wait for the scheduled time.
4. Confirm one notification appears.
5. Record the dose before a later schedule time and confirm the later dose-due notification is suppressed.

## Missed-Dose Reminder

1. Confirm missed-dose notifications are enabled.
2. Set a schedule time a few minutes in the future.
3. Do not record a take for that dose.
4. Confirm one private missed-dose notification appears after the grace period.
5. Confirm repeating the job for the same scheduled occurrence does not send another notification.
6. Record a take inside the expected window for a new scheduled occurrence and confirm no missed-dose notification is sent.

## Low-Stock Reminder

1. Confirm low-stock notifications are enabled.
2. Set medication stock just above the reorder threshold.
3. Record a take that crosses the threshold.
4. Confirm one private low-stock notification appears.
5. Confirm repeating the job for the same stock event does not send another notification.
6. Restock the medication above the threshold, cross the threshold again, and confirm a new notification can be sent.

## Privacy And Preference Checks

1. Confirm visible notification text does not include person name, medication name, or dose details for missed-dose and low-stock notifications.
2. Disable missed-dose notifications and confirm overdue doses do not send push notifications.
3. Disable low-stock notifications and confirm threshold crossings do not send push notifications.
4. Remove the active browser subscription and confirm missed-dose and low-stock jobs record/log a skip without raising.
5. Expire or invalidate a stored subscription and confirm delivery cleanup removes it without stopping delivery to other subscriptions.
