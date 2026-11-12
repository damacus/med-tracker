import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const read = path => readFileSync(path, 'utf8');
const templates = readdirSync('assets/views/administration', { recursive: true })
  .filter(path => path.endsWith('.html'));

test('medicine lookup uses a ranked list with accessible reorder controls', () => {
  const page = read('assets/views/platform/settings.html');
  assert.doesNotMatch(page, /Some previously saved sources/);
  assert.match(page, /data-lookup-priority/);
  assert.match(page, /Move {{ source.name }} up/);
  assert.match(page, /Move {{ source.name }} down/);
  assert.doesNotMatch(page, /type="text"[^>]*medicine_lookup_source_priority/);
});

test('platform settings identifies inactive medicine lookup service URLs', () => {
  const page = read('assets/views/platform/settings.html');
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
  assert.match(read('assets/views/administration/household.html'), /<a[^>]+href="\/households\/{{ slug }}\/admin{{ language_suffix }}">{{ labels.admin.users.form.cancel }}/);
  assert.match(read('assets/views/administration/role.html'), /href="{{ member_return_url }}">{{ labels.admin.users.form.cancel }}/);
  assert.match(read('assets/views/administration/bulk_members.html'), /href="{{ return_url }}">{{ bulk_labels.back }}/);
});

test('administration switcher keeps six ordinary localised route links and current-page semantics', () => {
  const navigation = read('assets/views/administration/navigation.html');
  assert.match(navigation, /<nav[^>]+class="admin-navigation section-switcher"/);
  assert.doesNotMatch(navigation, /role="tab(?:list)?"|aria-selected|\bbtn(?:-neutral|-ghost)?\b/);
  const links = [...navigation.matchAll(/<a\b[^>]+>/g)].map(match => match[0]);
  const paths = ['', '/users', '/carer_relationships', '/invitations', '/audit_logs', '/household/edit'];
  assert.equal(links.length, paths.length);
  for (const [index, link] of links.entries()) {
    const route = `/households/{{ slug }}/admin${paths[index]}{{ language_suffix }}`;
    assert.ok(link.includes(index === 1 ? route : `href="${route}"`));
    assert.match(link, /class="section-switcher-item"/);
    assert.match(link, /{% if admin_section == '[a-z]+' %}aria-current="page"{% endif %}/);
  }
});

test('member detail carries its validated filtered list return through the Members switcher', () => {
  const member = read('assets/views/administration/member.html');
  assert.match(member, /{% set members_navigation_url = member_return_url %}\s*{% include "administration\/navigation\.html" %}/);
  const navigation = read('assets/views/administration/navigation.html');
  assert.match(navigation, /href="{% if members_navigation_url %}{{ members_navigation_url }}{% else %}\/households\/{{ slug }}\/admin\/users{{ language_suffix }}{% endif %}"/);
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

test('overview retains six actions in shared content-sized rows and removes its recent activity feed', () => {
  const overview = read('assets/views/administration/index.html');
  const styles = read('assets/static/css/administration.css');
  assert.equal([...overview.matchAll(/class="admin-action-row shell-panel"/g)].length, 6);
  assert.match(overview, /href="\/households\/{{ slug }}\/admin\/audit_logs{{ language_suffix }}"/);
  assert.doesNotMatch(overview, /recent-activity-heading|recent_activity|admin-activity|overview_labels\.recent|overview_labels\.no_recent/);
  assert.match(styles, /@media \(min-width:768px\)[\s\S]*\.admin-action-groups > section[^}]+grid-template-rows:subgrid/);
  assert.match(styles, /\.admin-action-list[^}]+grid-template-rows:subgrid/);
  assert.doesNotMatch(styles.match(/\.admin-action-row \{([^}]+)\}/)?.[1] || '', /(?:^|;)\s*(?:max-)?height:/);
});

test('care deactivation stays a content-width touch target with its existing confirmation', () => {
  const relationships = read('assets/views/administration/relationships.html');
  assert.match(relationships, /<button class="btn btn-outline admin-relationship-action" type="button" data-dialog-open="deactivate-{{ relationship.id }}"/);
  assert.match(relationships, /{% if relationship.active %}[\s\S]*<dialog id="deactivate-{{ relationship.id }}"/);
  assert.match(relationships, /action="\/households\/{{ slug }}\/admin\/carer_relationships\/{{ relationship.id }}\/deactivate" method="post"/);
  assert.match(relationships, /name="authenticity_token" value="{{ authenticity_token }}"/);
  assert.match(relationships, /data-dialog-close/);
  assert.match(read('assets/static/css/administration.css'), /\.admin-relationship-action \{[^}]*width:fit-content;[^}]*min-height:2\.75rem/);
});

test('all invitation sections share full-width panels and gutters without changing form endpoints', () => {
  const invitations = read('assets/views/invitations/index.html');
  const styles = read('assets/static/css/administration.css');
  assert.match(invitations, /class="admin-page admin-invitations-page"/);
  assert.doesNotMatch(invitations.slice(0, invitations.indexOf('<h1')), /<a\b/);
  for (const [action, method] of [['invitations', 'post'], ['dependent-sharing-invitations', 'post'], ['invitations', 'get']]) {
    assert.ok(invitations.includes(`action="/households/{{ slug }}/admin/${action}" method="${method}"`));
  }
  assert.match(invitations, /class="admin-filter shell-panel grid/);
  assert.doesNotMatch(styles, /\.admin-page[^\n]*(?:max-width:[^\n]*#invitation-form|#invitation-form[^\n]*max-width)/);
  assert.match(styles, /\.admin-invitations-page > \.card > \.card-body \{ padding:1\.25rem; \}/);
  assert.match(styles, /\.admin-filter \{ padding:1\.25rem; \}/);
  assert.match(styles, /\.admin-invitations \.card-body \{[^}]*padding:1\.25rem;/);
});

for (const [page, description] of [
  ['administration/index.html', 'overview_labels.summary'],
  ['administration/users.html', 'admin_labels.overview.review_access'],
  ['administration/relationships.html', 'labels.admin.carer_relationships.index.subtitle'],
  ['invitations/index.html', 'invite.subtitle'],
  ['administration/audit_logs/index.html', 'admin_labels.audit.description'],
  ['administration/household.html', 'labels.admin.households.subtitle'],
]) {
  test(`${page} orders its heading, one translated description and section switcher consistently`, () => {
    const source = read(`assets/views/${page}`);
    const navigation = source.indexOf('{% include "administration/navigation.html" %}');
    assert.ok(navigation > 0, `${page} switcher missing`);
    const header = source.slice(source.indexOf('<section'), navigation).trim();
    assert.match(header, /^<section[^>]*>\s*<h1\b/);
    assert.doesNotMatch(header, /administration\/context\.html/);
    assert.equal([...header.matchAll(/<h1\b/g)].length, 1);
    assert.equal([...header.matchAll(/<p\b/g)].length, 1);
    assert.match(header, /<\/h1>\s*<p\b[^>]*>[\s\S]*<\/p>$/);
    assert.ok(header.includes(`{{ ${description} }}`), `${page} translated description missing`);
  });
}
