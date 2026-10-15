# Rails reference (read-only)

Rails is read-only reference code. Do not implement Rust changes here or run Rails
verification for Rust-only work. The historical guidance below describes Rails
conventions, not permission to change Rails or requirements for Rust parity.

## Rails rollback stack

- Ruby 4.0.6
- Rails 8.1.3
- PostgreSQL 18
- Bundler 4.0.3
- RSpec, Capybara, VCR, Rails fixtures
- Playwright 1.61
- RubyUI, Phlex, Hotwire, Propshaft
- RuboCop

## Code and Architecture

### Security protocols and primitives

Do not implement security protocols or cryptographic primitives from scratch
when a mature, maintained library or framework capability exists. This includes
OAuth/OIDC, PKCE, WebAuthn/passkeys, JWT/JWK, signature verification, password
hashing, token/session validation, CSRF protection, authentication challenges,
nonces, state, key parsing and algorithm negotiation.

Before introducing custom security-sensitive protocol code:

1. Search project dependencies and the relevant ecosystem for established implementations.
2. Prefer the mature implementation.
3. Document the exact limitation if it cannot satisfy the requirement.
4. Record why custom implementation is necessary.
5. Add standards/interoperability and negative security tests.
6. Keep the custom implementation as small as possible.

Passing tests does not justify bespoke security code. Do not replace a
battle-tested library with custom logic for perceived simplicity, dependency
reduction, token savings or implementation convenience.

Before implementing a non-trivial infrastructure or protocol capability, check
whether an established dependency or framework feature already provides it.
Prefer adopting and configuring proven implementations unless project
requirements materially prevent doing so.

### Application code

- Use RuboCop as the source of truth for Ruby style.
- Prefer clear names, small private methods, guard clauses, and Enumerable methods where they improve readability.
- Keep controllers focused on HTTP concerns; put business logic in models, POROs, or service objects.
- Use Phlex components in `rails/app/components/`.
- Add nil-safety guards in policy/component code when records or associations may be absent.
- Use `respond_to?` guards where a policy record may be a Class, such as `new` actions.

## Views and UI

- All views are Ruby/Phlex files. Do not create `.erb` files.
- Use RubyUI components when an equivalent component exists, especially for headings, text, links, buttons, forms, cards, tables, dialogs, badges, avatars, separators, popovers, tooltips, and calendar/date inputs.
- Keep UI accessible: keyboard access, labels, alt text, visible focus states, WCAG AA text contrast, descriptive link text, associated error messages, table headers, dismissible focus-trapping dialogs, and logical heading hierarchy.
- For UI work, verify through the actual UI with browser automation and capture desktop/mobile screenshots when the change is visible.
- Fetch current RubyUI docs with Context7 when component API details are unclear.

## Data Access and Performance

- Do not execute database queries inside view components or loops.
- Eager-load associations used by views with `includes`, `preload`, or `eager_load` in the controller/query boundary.
- Pass preloaded associations or query results into components instead of re-querying in nested methods.
- Use `size` on loaded collections instead of `count`.
- Use database filtering/sorting (`where`, `order`) for unloaded ActiveRecord relations.
- Use in-memory filtering (`select`, `reject`) only on already-loaded/eager-loaded collections.
- Prefer `exists?` over `find_by` when only checking existence.
- Wrap related writes in `ActiveRecord::Base.transaction` and use bang persistence methods inside transactions.


Use the Rails tasks documented in its Taskfile only for explicitly authorised Rails work.
