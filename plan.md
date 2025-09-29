# PLAN

## Medicine Supply

- Implement inventory tracking on each `Medicine` with reorder thresholds and current stock visibility.
- Build ordering workflow to capture supplier, quantity, expected arrival, and status.
- Record lead time analytics from completed orders to forecast when to reorder.

## Administration & Roles

- Attach `MedicationTake` entries to the administering `User` and expose an audit log UI.
- Enforce role-based permissions aligned with `User.role` (`child`, `carer`, `admin`) across controllers and views.
- Deliver admin dashboards to manage users, medicines, orders, and role assignments.

## Medicine Catalog

- Seed the database with core medicines (Laxido, Movicol, adult vitamins, child vitamins) including dosage defaults.
- Provide UI affordances to add recommended medicines quickly without re-entering defaults.

## Notifications

- Schedule reminders for upcoming doses using `SolidQueue` and deliver via email or push.
- Trigger alerts for low inventory, expired medicines, and overdue orders.

## PWA & UX

- Generate transparent PWA icons (`icon-192.png`, `icon-512.png`) and update manifest and service worker references.
- Improve install prompts and offline handling for the PWA experience.

## Documentation & Testing

- Expand `README.md` with role capabilities, PWA setup instructions, and roadmap context.
- Add system and request specs covering admin dashboards, inventory flows, and notification delivery.
