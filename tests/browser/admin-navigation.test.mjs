import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const read = path => readFileSync(path, 'utf8');
const templates = readdirSync('assets/views/administration', { recursive: true })
  .filter(path => path.endsWith('.html'));

test('medicine lookup uses a ranked list with accessible reorder controls', () => {
  const page = read('assets/views/administration/service/settings.html');
  assert.doesNotMatch(page, /Some previously saved sources/);
  assert.match(page, /data-lookup-priority/);
  assert.match(page, /Move {{ source.name }} up/);
  assert.match(page, /Move {{ source.name }} down/);
  assert.doesNotMatch(page, /type="text"[^>]*medicine_lookup_source_priority/);
});

test('service settings identifies inactive medicine lookup service URLs', () => {
  const page = read('assets/views/administration/service/settings.html');
  assert.doesNotMatch(page, /type="url"/);
  assert.doesNotMatch(page, /name="medicine_lookup_(?:base|token)_url" type="url"/);
});

test('household administration headings have no redundant standalone back links', () => {
  for (const path of templates) {
    const source = read(`assets/views/administration/${path}`);
    if (!source.includes('<h1')) continue;
    const header = source.slice(0, source.indexOf('<h1'));
    assert.doesNotMatch(header, /<a\b/, `${path} has a link above its heading`);
  }
  assert.match(read('assets/views/administration/role.html'), /href="{{ member_return_url }}">{{ labels.admin.users.form.cancel }}/);
  assert.match(read('assets/views/administration/bulk_members.html'), /href="{{ return_url }}">{{ bulk_labels.back }}/);
});

test('administration switcher uses a shared route bar and gates service destinations', () => {
  const navigation = read('assets/views/administration/navigation.html');
  assert.match(navigation, /<nav[^>]+class="admin-navigation section-switcher"/);
  assert.match(navigation, /shell\.system_administrator/);
  assert.match(navigation, /shell\.can_manage_household/);
  assert.doesNotMatch(navigation, /\/platform|\/admin\/invitations|\/admin\/household\/edit/);
  const links = [...navigation.matchAll(/<a\b[^>]+>/g)].map(match => match[0]);
  assert.equal(links.length, 8);
  for (const link of links) {
    assert.match(link, /class="section-switcher-item"/);
    assert.match(link, /aria-current="page"/);
  }
  for (const route of ['settings', 'nhs-dmd', 'support', 'owner-recovery']) {
    assert.ok(navigation.includes(`href="/administration/${route}"`));
  }
});

test('member detail carries its validated filtered list return through the Members switcher', () => {
  const member = read('assets/views/administration/member.html');
  assert.match(member, /{% set members_navigation_url = member_return_url %}\s*{% include "administration\/navigation\.html" %}/);
  const navigation = read('assets/views/administration/navigation.html');
  assert.ok(navigation.includes('{% if members_navigation_url %}{{ members_navigation_url }}'));
  assert.doesNotMatch(member.slice(0, member.indexOf('<h1')), /<a\b/);
  assert.doesNotMatch(read('assets/views/administration/role.html'), /members_navigation_url/);
  assert.match(read('assets/views/administration/role.html'), /href="{{ member_return_url }}">{{ labels.admin.users.form.cancel }}/);
});

test('profile and administration use one raised switcher with visible focus and touch targets', () => {
  const shared = read('assets/static/css/section-switcher.css');
  assert.match(read('assets/static/css/app.css'), /@import "\.\/section-switcher\.css";/);
  assert.doesNotMatch(read('assets/static/css/profile.css'), /\.profile-tabs/);
  assert.match(shared, /\.section-switcher, \.profile-tabs/);
  assert.match(shared, /\.section-switcher \.section-switcher-item, \.profile-tabs \.profile-tab/);
  assert.match(shared, /\[aria-current="page"\]/);
  assert.match(shared, /\[aria-selected="true"\]/);
  assert.match(shared, /min-height: 2\.75rem/);
  assert.match(shared, /:focus-visible/);
  assert.match(shared, /overflow-wrap: normal/);
  assert.doesNotMatch(read('assets/static/css/administration.css'), /\.admin-navigation \.btn/);
});

test('overview contains household actions and service status cards without duplicate links', () => {
  const overview = read('assets/views/administration/index.html');
  const service = read('assets/views/administration/service/status.html');
  const households = read('assets/views/administration/household_cards.html');
  assert.match(overview, /include "administration\/household_cards\.html"/);
  assert.match(households, /Rename household/);
  assert.match(households, /Manage users/);
  assert.match(overview, /{% if service_status %}{% include "administration\/service\/status\.html" %}/);
  assert.doesNotMatch(service, /<a\b|<button\b|View all accounts/);
  assert.match(service, /Last successful import/);
  assert.match(service, /Open requests/);
  assert.match(service, /Active sessions/);
  assert.match(service, /Managed by system configuration/);
});

test('care deactivation stays a content-width touch target with its existing confirmation', () => {
  const relationships = read('assets/views/administration/relationships.html');
  assert.match(relationships, /<button class="btn btn-outline admin-relationship-action" type="button" data-dialog-open="deactivate-{{ relationship.id }}"/);
  assert.match(relationships, /{% if relationship.effective_status == "active" %}[\s\S]*<dialog id="deactivate-{{ relationship.id }}"/);
  assert.match(relationships, /action="\/households\/{{ slug }}\/admin\/carer_relationships\/{{ relationship.id }}\/deactivate" method="post"/);
  assert.match(relationships, /name="authenticity_token" value="{{ authenticity_token }}"/);
  assert.match(relationships, /data-dialog-close/);
  assert.match(read('assets/static/css/administration.css'), /\.admin-relationship-action \{[^}]*width:fit-content;[^}]*min-height:2\.75rem/);
});

test('all invitation sections share full-width panels and gutters without changing form endpoints', () => {
  const invitations = read('assets/views/administration/invitation_panel.html');
  const styles = read('assets/static/css/administration.css');
  assert.match(read('assets/views/administration/users.html'), /administration\/invitation_panel\.html/);
  for (const [action, method] of [['invitations', 'post'], ['dependent-sharing-invitations', 'post'], ['invitations', 'get']]) {
    assert.ok(invitations.includes(`action="/households/{{ slug }}/admin/users/${action}" method="${method}"`));
  }
  assert.match(invitations, /class="admin-filter shell-panel grid/);
  assert.doesNotMatch(styles, /\.admin-page[^\n]*(?:max-width:[^\n]*#invitation-form|#invitation-form[^\n]*max-width)/);
  assert.match(styles, /\.admin-invitations-page > \.card > \.card-body \{ padding:1\.25rem; \}/);
  assert.match(styles, /\.admin-filter \{ padding:1\.25rem; \}/);
  assert.match(styles, /\.admin-invitations \.card-body \{[^}]*padding:1\.25rem;/);
});

for (const page of [
  "index", "users", "relationships", "user_invitations", "audit_logs/index",
  "service/index", "service/users", "service/settings", "service/nhs-dmd",
  "service/support", "service/owner_recovery",
]) {
  test(`${page} starts with the Administration switcher and a visually hidden heading`, () => {
    const source = read(`assets/views/administration/${page}.html`);
    const section = source.slice(source.indexOf("<section"));
    assert.match(section, /^<section[^>]*>\s*<h1 class="sr-only">[^\n]+<\/h1>\s*{% include "administration\/navigation.html" %}/);
  });
}
