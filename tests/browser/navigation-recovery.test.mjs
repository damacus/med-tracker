import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = path => readFileSync(path, 'utf8');

test('household chrome requires the complete authorised shell rather than a slug', () => {
  const base = read('assets/views/layouts/base.html');
  assert.doesNotMatch(base, /{% if slug(?: %}| or standalone_navigation)/);
  assert.match(base, /{% if shell %} class="app-household"/);
  assert.match(base, /{% if shell %}{% include "components\/app-sidebar.html" %}/);
  const navigation = read('assets/views/components/app-navigation.html');
  assert.doesNotMatch(navigation, /{% if slug %}<a[^>]+data-global-search-trigger/);
  const support = read('src/controllers/platform/support.rs').split('fn render_read(')[1].split('\nfn ')[0];
  assert.doesNotMatch(support, /data\["slug"\]/);
});

test('optional shell decoration classifies authority failures without swallowing unavailable errors', () => {
  const source = read('src/controllers/shell.rs');
  const fallback = source.split('fn optional_shell_error(')[1]?.split('\npub(crate) struct ShellView')[0];
  assert.ok(fallback, 'typed optional-shell classification is missing');
  for (const denial of ['Unauthenticated', 'Forbidden', 'NotFound', 'InsufficientScope']) {
    assert.ok(fallback.includes(`AuthenticationError::${denial}`), `${denial} is unclassified`);
  }
  assert.match(fallback, /=> Ok\(None\)/);
  assert.match(fallback, /AuthenticationError::Unavailable => Err\(Box::new\(authentication_error\(error\)\)\)/);
  assert.doesNotMatch(fallback, /_\s*=>\s*Ok\(None\)|\.status\(\)/);
  const strictLoad = source.split('pub(super) async fn load(')[1].split('\nasync fn load_authorized(')[0];
  assert.match(strictLoad, /begin\(ctx, session, slug, request_id\)[\s\S]*map_err\(authentication_error\)/);
});

test('shared sidebar and shortcut administration URLs retain the validated page language suffix', () => {
  const render = read('src/controllers/shell.rs').split('impl ViewRenderer for ShellView')[1];
  assert.match(render, /data\s*\.get\("language_suffix"\)/);
  assert.match(render, /link\.id == "administration"[\s\S]*link\.href\.push_str\(language_suffix\)/);
  assert.ok(render.indexOf('link.href.push_str(language_suffix)') < render.indexOf('let sidebar ='));
  assert.doesNotMatch(render, /query\(\)|parts\.uri/);
});

test('profile reader dispatch retains its selected presentation when optional frame authentication fails', () => {
  const shell = read('src/controllers/shell.rs');
  const accessor = shell.split('fn profile_presentation(')[1].split('\n    pub(crate) fn profile_destination')[0];
  assert.match(accessor, /self\.presentation\.as_ref\(\)/);
  assert.doesNotMatch(accessor, /self\.flow/);
  const frame = read('src/controllers/profile/dialogs.rs').split('pub(crate) async fn load(')[1].split('pub(crate) fn render(')[0];
  assert.match(frame, /Result<Option<Self>, Box<Response>>/);
  assert.match(frame, /AuthenticationError::Unauthenticated[\s\S]*return Ok\(None\)/);
  assert.match(frame, /Err\(error @ AuthenticationError::Unavailable\)\s*=>\s*\{?\s*return Err\(Box::new\(authentication_error\(error\)\)\)/);
  assert.doesNotMatch(frame, /Err\(_\).*Ok\(None\)/);
  const history = read('src/controllers/profile/account_profile/history.rs');
  assert.match(history, /Err\(error\) => return refused\(ctx, None, request_id, authentication_error\(error\)\)\.await/);
  assert.match(history, /deny_personal_history/);
});
