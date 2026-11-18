const sections = ['Overview', 'Users', 'Care relationships', 'Audit', 'Registration and medicine settings', 'NHS dm+d', 'Support access', 'Owner recovery'];
const state = { role: 'manager', section: 'Overview', scope: 'household', query: '', users: [
  { name: 'Amelia Hart', email: 'amelia@example.test', household: 'Willow House', role: 'Manager', status: 'Active', protected: 'You cannot remove your own manager access.' },
  { name: 'Noah Hart', email: 'noah@example.test', household: 'Willow House', role: 'Member', status: 'Active' },
  { name: 'Maya Patel', email: 'maya@example.test', household: 'Willow House', role: 'Member', status: 'Active' },
  { name: 'Oliver Reed', email: 'oliver@example.test', household: 'Oak House', role: 'Manager', status: 'Active', protected: 'The last manager of a household cannot be removed.' },
  { name: 'Sam Morgan', email: 'sam@example.test', household: 'No household', role: 'System administrator', status: 'Active', protected: 'The last system administrator cannot be removed.' }
], invitations: [{email:'alex@example.test', role:'Member', status:'Pending'}], relationships: ['Amelia cares for Maya', 'Noah cares for Maya'], medicines: ['Imported medicines', 'NHS medicines dictionary', 'Open Products Facts', 'Curated medicines'], support: 'No active access', registration: true, importProgress: null, importError: '', importHistory: [{date:'12 November 2026', status:'Complete', detail:'148,203 entries imported'}, {date:'4 November 2026', status:'Failed', detail:'Invalid archive; existing catalogue retained'}], recoveryResult: '' };
const nav = document.querySelector('#admin-nav');
const content = document.querySelector('#content');
const notice = document.querySelector('#notice');
const registrationPreview = document.querySelector('#registration-preview');
const registrationOverride = () => registrationPreview.value === 'saved' ? null : registrationPreview.value === 'open';
registrationPreview.addEventListener('change', render);
const system = () => state.role.startsWith('system');
const manager = () => state.role === 'manager' || state.role === 'system';
const visible = () => state.role === 'member' ? [] : sections.filter(s => s === 'Overview' || s === 'Users' || (manager() && ['Care relationships','Audit'].includes(s)) || (system() && ['Registration and medicine settings','NHS dm+d','Support access','Owner recovery'].includes(s)));
const householdNames = new Map([['Willow House', 'Willow House'], ['Oak House', 'Oak House']]);
const managedHouseholds = () => state.role === 'system' ? [...householdNames.keys()] : state.role === 'manager' ? ['Willow House'] : [];
let activeHousehold = 'Willow House';
const householdPicker = document.querySelector('#managed-household');
const householdName = id => householdNames.get(id) ?? id;
const activeName = () => escapeHtml(householdName(activeHousehold));
const householdInvitations = id => state.invitations.filter(invite => (invite.household ?? 'Willow House') === id);
const householdRelationships = new Map([['Willow House', state.relationships], ['Oak House', []]]);
function selectHousehold(id) {
  if (!managedHouseholds().includes(id)) return;
  activeHousehold = id;
  state.scope = 'household';
  state.query = '';
  notice.hidden = true;
}
householdPicker.addEventListener('change', () => {
  selectHousehold(householdPicker.value);
  render();
});
const escapeHtml = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const button = (label, action, extra = '') => `<button class="button ${extra}" data-action="${action}">${label}</button>`;
const heading = (title, description, action = '') => `<div class="section-heading"><div><h2>${title}</h2><p>${description}</p></div>${action}</div>`;
function flash(message) { notice.textContent = message; notice.hidden = false; }
function render() {
  const available = visible();
  const managed = managedHouseholds();
  if (!managed.includes(activeHousehold)) activeHousehold = managed[0] ?? '';
  const householdView = ['Care relationships', 'Audit'].includes(state.section)
    || (state.section === 'Users' && state.scope === 'household');
  householdPicker.closest('label').hidden = managed.length < 2 || !householdView;
  householdPicker.replaceChildren(...managed.map(id => new Option(householdName(id), id, false, id === activeHousehold)));
  if (!available.includes(state.section)) state.section = 'Overview';
  nav.innerHTML = available.map(s => `<button data-section="${s}" ${s === state.section ? 'aria-current="page"' : ''}>${s === 'Registration and medicine settings' ? 'Service settings' : s}</button>`).join('');
  registrationPreview.closest('label').hidden = state.section !== 'Registration and medicine settings' || !system();
  nav.hidden = !available.length;
  const current = nav.querySelector('[aria-current="page"]');
  if (current && nav.scrollWidth > nav.clientWidth) {
    const start = current.getBoundingClientRect().left - nav.getBoundingClientRect().left + nav.scrollLeft;
    nav.scrollLeft = start - (nav.clientWidth - current.offsetWidth) / 2;
  }
  if (!available.length) { content.innerHTML = '<div class="card access-denied"><h2>Administration is unavailable</h2><p>Household members can manage their own details in Profile. Ask a household manager if you need help.</p><a class="button" href="profile.html">Go to Profile</a></div>'; return; }
  const pages = { 'Overview': overview, 'Users': users, 'Care relationships': relationships, 'Audit': audit, 'Registration and medicine settings': settings, 'NHS dm+d': dmd, 'Support access': support, 'Owner recovery': recovery };
  content.innerHTML = pages[state.section]();
}
function overview() {
  const cards = managedHouseholds().map(id => {
    const name = escapeHtml(householdName(id));
    const memberCount = state.users.filter(user => user.household === id).length;
    const pending = householdInvitations(id).filter(invite => invite.status === 'Pending').length;
    return `<article class="card household-card">
      <h3>${name}</h3>
      <p class="muted">${memberCount} ${memberCount === 1 ? 'member' : 'members'} · ${pending} pending ${pending === 1 ? 'invitation' : 'invitations'}</p>
      <div class="actions">
        <button class="button" data-action="rename-household:${id}" aria-label="Rename household: ${name}">Rename household</button>
        <a class="button" href="#users" data-go="Users" data-household="${id}" data-user-scope="household" aria-label="Manage users in ${name}">Manage users</a>
      </div>
    </article>`;
  }).join('');
  return `${heading('Overview', 'See the households you manage and what needs your attention.')}
    ${cards ? `<section class="household-overview" aria-labelledby="households-title"><h3 id="households-title">Your households</h3><div class="grid">${cards}</div></section>` : ''}
    ${system() ? serviceOverview() : ''}`;
}

function lastSuccessfulImport() {
  return state.importHistory.find(item => item.status === 'Complete');
}

function importStatus() {
  if (state.importProgress !== null) return `Import in progress · ${state.importProgress}%`;
  if (state.importHistory[0]?.status === 'Failed') return 'Latest import failed';
  return lastSuccessfulImport() ? 'Last import completed' : 'No catalogue imported';
}

function serviceOverview() {
  const lastImport = lastSuccessfulImport();
  const openRequests = Number(['Awaiting consent', 'Consent received'].includes(state.support));
  const activeSessions = Number(state.support === 'Active · read-only');
  const override = registrationOverride();
  return `<section class="household-overview" aria-labelledby="service-status-title">
    <h3 id="service-status-title">Service status</h3>
    <div class="grid">
      <article class="card">
        <h4>NHS dm+d</h4>
        <p id="overview-import-status"><strong>${importStatus()}</strong></p>
        <p class="muted" id="overview-import-date">Last successful import: ${lastImport ? escapeHtml(lastImport.date) : 'None yet'}</p>
      </article>
      <article class="card">
        <h4>Support access</h4>
        <dl class="service-counts">
          <div><dt>Open requests</dt><dd>${openRequests}</dd></div>
          <div><dt>Active sessions</dt><dd>${activeSessions}</dd></div>
        </dl>
      </article>
      <article class="card">
        <h4>Registration</h4>
        <p><strong>${(override ?? state.registration) ? 'Open' : 'Invite-only'}</strong></p>
        ${override === null ? '' : '<p class="muted">Managed by system configuration</p>'}
      </article>
    </div>
  </section>`;
}
function users() {
  if (!manager()) state.scope = 'all';
  const list = state.users.filter(u => (state.scope === 'all' || u.household === activeHousehold) && `${u.name} ${u.email} ${householdName(u.household)}`.toLowerCase().includes(state.query.toLowerCase()));
  const invited = manager() && state.scope === 'household'
    ? householdInvitations(activeHousehold).filter(invitation => invitation.email.toLowerCase().includes(state.query.toLowerCase()))
    : [];
  const invitationRows = invited.map(invitation => {
    const index = state.invitations.indexOf(invitation);
    const pending = invitation.status === 'Pending';
    const actions = pending ? button('Resend', `resend:${index}`) + button('Revoke', `revoke:${index}`) : '';
    return `<tr><td><strong>${escapeHtml(invitation.email)}</strong><small>Invitation</small></td><td>${escapeHtml(invitation.role)}</td><td><span class="badge ${pending ? 'warning' : 'grey'}">${pending ? 'Invitation pending' : 'Invitation revoked'}</span></td><td><div class="inline-actions">${actions}</div></td></tr>`;
  }).join('');
  return `${heading('Users', 'Find a person and manage the access available to you.', manager() ? button('Invite a person', 'invite', 'primary') : '')}${system() && manager() ? `<div class="tabs" role="tablist" aria-label="User scope"><button role="tab" data-scope="household" aria-selected="${state.scope === 'household'}">This household</button><button role="tab" data-scope="all" aria-selected="${state.scope === 'all'}">All accounts</button></div>` : ''}<div class="scope-note"><strong>${state.scope === 'household' ? `This household · ${activeName()}` : 'All accounts · system administration'}</strong><br>${state.scope === 'household' ? `Manage household membership and roles for ${activeName()}.` : 'Search accounts across the service. Account actions do not change household roles.'}</div><div class="toolbar"><label class="sr-only" for="user-search">Search users</label><input id="user-search" class="search" type="search" placeholder="Search name, email or household" value="${escapeHtml(state.query)}"><span class="muted">${list.length} ${list.length === 1 ? 'person' : 'people'}${invited.length ? ` · ${invited.length} ${invited.length === 1 ? 'invitation' : 'invitations'}` : ''}</span></div><div class="panel table-wrap"><table><thead><tr><th>Person</th><th>${state.scope === 'all' ? 'Household' : 'Household role'}</th><th>${state.scope === 'household' ? 'Membership or invitation status' : 'Account status'}</th><th><span class="sr-only">Action</span></th></tr></thead><tbody>${list.map(u => `<tr><td><strong>${escapeHtml(u.name)}</strong><small>${escapeHtml(u.email)}</small></td><td>${escapeHtml(state.scope === 'all' ? householdName(u.household) : u.role)}</td><td><span class="badge ${(state.scope === 'household' ? u.membership || 'Active' : u.status) === 'Active' ? '' : 'grey'}">${escapeHtml(state.scope === 'household' ? u.membership || 'Active' : u.status)}</span></td><td>${button('Manage', `edit:${state.users.indexOf(u)}`)}</td></tr>`).join('')}${invitationRows}</tbody></table>${list.length || invited.length ? '' : '<div class="empty">No users or invitations match your search.</div>'}</div>`;
}
function relationships() { return `${heading('Care relationships', 'Choose who can help manage another person’s medicines.', button('Add relationship','add-relationship','primary'))}<div class="panel panel-body">${householdRelationships.get(activeHousehold).map((r,i) => `<div class="list-row"><div class="details"><strong>${escapeHtml(r)}</strong><small>${activeName()}</small></div>${button('Remove',`remove-relationship:${i}`)}</div>`).join('') || '<p class="muted">No care relationships yet.</p>'}</div>`; }
function audit() { return `${heading('Audit', `Recent activity in ${activeName()}.`)}${activeHousehold === 'Willow House' ? `<div class="panel table-wrap"><table><thead><tr><th>When</th><th>Actor</th><th>Activity</th><th>Household</th></tr></thead><tbody><tr><td>Today, 09:42</td><td>Amelia Hart</td><td>Invited alex@example.test</td><td>${activeName()}</td></tr><tr><td>Yesterday, 16:20</td><td>Amelia Hart</td><td>Updated a care relationship</td><td>${activeName()}</td></tr></tbody></table></div>` : '<p class="card muted">No recent activity in this household.</p>'}`; }
function renameHousehold(id) {
  if (!managedHouseholds().includes(id)) return;
  const previous = escapeHtml(householdName(id));
  dialog(`Rename ${previous}`, `<div class="field"><label for="rename-household">Household name</label><input id="rename-household" name="name" value="${previous}" maxlength="100" required pattern=".*\\S.*" autocomplete="off"></div>`, 'Save name', data => {
    const name = String(data.get('name')).trim();
    householdNames.set(id, name);
    render();
    flash(`Household renamed to ${name}.`);
  });
}
function settings() {
  const override = registrationOverride();
  const locked = override !== null;
  const open = override ?? state.registration;
  return `${heading('Registration and medicine settings', 'Manage registration access and medicine lookup.') }
    <div class="split">
      <section class="card registration-card${locked ? ' is-locked' : ''}" aria-labelledby="registration-title">
        <h3 id="registration-title">Registration</h3>
        <p class="muted">${open ? 'Anyone can create an account.' : 'People can only register by invitation.'}</p>
        <fieldset ${locked ? 'disabled' : ''}>
          <legend class="sr-only">Registration settings</legend>
          <div class="list-row">
            <strong class="details">${open ? 'Registration open' : 'Registration closed'}</strong>
            ${button(open ? 'Close registration' : 'Open registration', 'toggle-registration')}
          </div>
        </fieldset>
        ${locked ? '<small>Managed by system configuration</small>' : ''}
      </section>
      <div class="card"><h3>Medicine lookup source order</h3><p class="muted">Change the order in which medicine sources are searched.</p>${state.medicines.map((m,i) => `<div class="list-row"><div class="details">${escapeHtml(m)}</div><button class="button" data-action="move:${i}:-1" aria-label="Move ${escapeHtml(m)} up" ${i === 0 ? 'disabled' : ''}>↑</button><button class="button" data-action="move:${i}:1" aria-label="Move ${escapeHtml(m)} down" ${i === state.medicines.length - 1 ? 'disabled' : ''}>↓</button></div>`).join('')}</div></div>`;
}
function dmd() { const progress = state.importProgress; return `${heading('NHS dm+d','Manage the medicine dictionary used by the service.')}<div class="stack"><div class="card"><span class="badge">Current</span><h3>Dictionary import</h3><p class="muted">Last successful import: ${escapeHtml(lastSuccessfulImport()?.date ?? 'None yet')}</p><form id="upload-form" novalidate><div class="field" style="margin-top:20px"><label for="upload">Choose a dictionary archive (.zip)</label><input type="file" id="upload" accept=".zip" aria-describedby="upload-error"><small id="upload-error" role="alert">${escapeHtml(state.importError)}</small></div><button class="button primary" ${progress !== null ? 'disabled' : ''}>Start mock import</button>${progress !== null ? `<div><p>Simulating import… ${progress}% · ${Math.round(progress * 1482.03).toLocaleString('en-GB')} of 148,203 entries</p><progress class="progress" max="100" value="${progress}">${progress}%</progress></div>` : ''}</form></div><div class="panel table-wrap"><table><thead><tr><th>Date</th><th>Status</th><th>Result</th></tr></thead><tbody>${state.importHistory.map(item => `<tr><td>${escapeHtml(item.date)}</td><td><span class="badge">${escapeHtml(item.status)}</span></td><td>${escapeHtml(item.detail)}</td></tr>`).join('')}</tbody></table></div></div>`; }
function support() { return `${heading('Support access','Review and request temporary access for troubleshooting.')}<div class="split"><div class="card"><span class="badge grey">${state.support}</span><h3 style="margin-top:14px">Temporary support access</h3><p class="muted">The account holder decides in Profile whether to consent. Active access is read-only and can be ended at any time.</p><div class="actions">${state.support === 'No active access' || state.support === 'Ended' ? button('Request access','request-support','primary') : ''}${state.support === 'Awaiting consent' ? button('Simulate consent received','simulate-consent') : ''}${state.support === 'Consent received' ? button('Start read-only session','start-support','primary') : ''}${state.support === 'Active · read-only' ? button('End access','end-support') : ''}</div>${state.support === 'Active · read-only' ? '<div class="scope-note" style="margin-top:16px"><strong>Read-only preview</strong><br>Account: Maya Patel · Willow House<br>Support can view account details during this simulated session. Editing is unavailable.</div>' : ''}</div><div class="card"><h3>How consent works</h3><p class="muted">The account holder reviews the request and can end access. Administrators cannot grant consent on their behalf. The simulation control represents a separate decision by the account holder.</p><a class="card-link" href="profile.html#security">Open Profile security →</a></div></div>`; }
function recovery() { return `${heading('Owner recovery','Help a household regain owner access.')}<div class="card"><div class="callout"><strong>Protected action</strong><br>Verify the request through the agreed recovery process before changing a household owner. Existing owner: Oliver Reed (Oak House); ownership cannot be removed without a replacement.</div>${state.recoveryResult ? `<div class="notice" style="margin-top:16px"><strong>Recovery completed · simulated</strong><br>${escapeHtml(state.recoveryResult)}<br>An owner recovery event was added to the mock audit trail.</div>` : ''}<form id="recovery-form" style="margin-top:20px"><div class="field"><label for="recovery-household">Household</label><select id="recovery-household"><option>Oak House</option><option>Willow House</option></select></div><div class="field"><label for="recovery-person">Account to make owner</label><input id="recovery-person" type="email" placeholder="person@example.test" required></div><button class="button primary">Review recovery</button></form></div>`; }
function dialog(title, body, submit, action) { const el = document.createElement('dialog'); el.className = 'dialog'; el.innerHTML = `<div class="dialog-inner"><h2>${title}</h2><form id="dialog-form">${body}<div class="actions"><button type="button" class="button" data-close>Cancel</button><button class="button primary">${submit}</button></div></form></div>`; document.body.append(el); el.querySelector('[data-close]').onclick = () => el.close(); el.addEventListener('close', () => el.remove()); el.querySelector('form').onsubmit = e => { e.preventDefault(); if (action(new FormData(e.target)) !== false) el.close(); }; el.showModal(); }
function invite() { const id = activeHousehold; dialog('Invite a person',`<p>Send an invitation to join ${activeName()}.</p><div class="field"><label for="invite-email">Email address</label><input id="invite-email" name="email" type="email" required></div><div class="field"><label for="invite-role">Household role</label><select id="invite-role" name="role"><option>Member</option><option>Manager</option></select></div>`,'Send invitation', data => { state.invitations.push({email:data.get('email'),role:data.get('role'),status:'Pending',household:id}); state.scope = 'household'; state.query = ''; flash('Invitation simulated. No email was sent.'); render(); }); }
function editUser(index) { const u = state.users[index]; const householdScope = state.scope === 'household'; const membership = u.membership || 'Active'; const signIn = u.signIn || 'Enabled'; const admin = u.admin || (u.role === 'System administrator' ? 'Yes' : 'No'); const select = (name, label, values, current) => `<div class="field"><label for="edit-${name}">${label}</label><select id="edit-${name}" name="${name}">${values.map(v => `<option ${v === current ? 'selected' : ''}>${v}</option>`).join('')}</select></div>`; dialog(`Manage ${escapeHtml(u.name)}`, `<p>${householdScope ? 'Household membership and account access are separate.' : 'Service account controls apply across all households.'}</p>${u.protected ? `<div class="callout">${u.protected}</div>` : ''}${householdScope ? `<h3 style="margin-top:16px">${activeName()} membership</h3>${select('membership','Membership state',['Active','Suspended'],membership)}${select('role','Household role',['Manager','Member'],u.role)}` : ''}${system() ? `<h3 style="margin-top:16px">Global account</h3>${select('account','Account state',['Active','Suspended'],u.status)}${select('signin','Sign-in',['Enabled','Disabled'],signIn)}${select('admin','System administrator',['No','Yes'],admin)}<div class="field"><label for="proof">Fresh proof for sensitive changes</label><input id="proof" name="proof" placeholder="Type VERIFY to simulate fresh proof"><small>Changing sign-in or administrator access requires fresh proof.</small></div>` : ''}`, 'Save changes', data => { const sensitive = system() && (data.get('signin') !== signIn || data.get('admin') !== admin); if (sensitive && data.get('proof') !== 'VERIFY') { flash('Sensitive changes require fresh proof. Enter VERIFY in the editor to simulate it.'); return false; } if (u.protected && ((householdScope && (data.get('role') !== u.role || data.get('membership') !== membership)) || (system() && (data.get('account') !== u.status || data.get('signin') !== signIn || data.get('admin') !== admin)))) { flash(u.protected); return false; } if (householdScope) { u.role = data.get('role'); u.membership = data.get('membership'); } if (system()) { u.status = data.get('account'); u.signIn = data.get('signin'); u.admin = data.get('admin'); } flash(`${u.name} updated in this prototype.`); render(); }); }
const sectionHash = section => section.toLowerCase().replaceAll(' ', '-');
function readLocation() { const requested = sections.find(s => sectionHash(s) === location.hash.slice(1)); state.section = requested && visible().includes(requested) ? requested : 'Overview'; render(); }
window.addEventListener('hashchange', readLocation);
document.addEventListener('click', e => { const link = e.target.closest('[data-section],[data-go]'); if (!link) return; e.preventDefault(); e.stopImmediatePropagation(); const section = link.dataset.section || link.dataset.go; if (link.dataset.household) selectHousehold(link.dataset.household); if (link.dataset.userScope) state.scope = link.dataset.userScope; state.query = ''; notice.hidden = true; if (location.hash === `#${sectionHash(section)}`) readLocation(); else location.hash = sectionHash(section); }, true);
document.addEventListener('keydown', e => { const tab = e.target.closest('[data-scope]'); if (!tab || !['ArrowLeft','ArrowRight','Home','End'].includes(e.key)) return; e.preventDefault(); const tabs = [...document.querySelectorAll('[data-scope]')]; const next = e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : (tabs.indexOf(tab) + (e.key === 'ArrowRight' ? 1 : -1) + tabs.length) % tabs.length; state.scope = tabs[next].dataset.scope; state.query = ''; render(); document.querySelector(`[data-scope="${state.scope}"]`).focus(); });
document.addEventListener('click', e => { const action = e.target.closest('[data-action]')?.dataset.action; if (action === 'simulate-consent') { state.support = 'Consent received'; flash('Consent received is simulated here; the account holder decides in Profile.'); render(); } if (action === 'start-support') { state.support = 'Active · read-only'; flash('Read-only support session started in this prototype.'); render(); } if (action === 'end-support') { state.support = 'Ended'; flash('Support access ended in this prototype.'); render(); } });
document.querySelector('#role').addEventListener('change', e => { state.role = e.target.value; state.scope = manager() ? 'household' : 'all'; state.query = ''; notice.hidden = true; if (location.hash) location.hash = ''; else readLocation(); });
document.addEventListener('input', e => { if (e.target.id === 'user-search') { const pos = e.target.selectionStart; state.query = e.target.value; render(); const input = document.querySelector('#user-search'); input.focus(); input.setSelectionRange(pos,pos); } });
document.addEventListener('click', e => { const scope = e.target.closest('[data-scope]'); if (scope) { state.scope = scope.dataset.scope; state.query = ''; render(); return; } const target = e.target.closest('[data-action]'); if (!target) return; const [action,a,b] = target.dataset.action.split(':'); if (action === 'rename-household') renameHousehold(a); if (action === 'invite') invite(); if (action === 'edit') editUser(Number(a)); if (action === 'resend') flash(`Invitation to ${state.invitations[a].email} resent in this prototype.`); if (action === 'revoke') { state.invitations[a].status = 'Revoked'; flash('Invitation revoked in this prototype.'); render(); } if (action === 'toggle-registration' && registrationOverride() === null) { state.registration = !state.registration; flash('Registration setting updated in this prototype.'); render(); } if (action === 'move') { const i = Number(a), j = i + Number(b); [state.medicines[i],state.medicines[j]] = [state.medicines[j],state.medicines[i]]; render(); } if (action === 'request-support') dialog('Request support access','<p>Enter the account holder’s email. They must approve the request in Profile security.</p><div class="field"><label for="support-email">Account holder email</label><input id="support-email" name="email" type="email" required></div>','Request access', data => { state.support = 'Awaiting consent'; flash(`Request for ${data.get('email')} simulated. No message was sent.`); render(); }); if (action === 'add-relationship') { const options = state.users.filter(user => user.household === activeHousehold).map(user => `<option>${escapeHtml(user.name)}</option>`).join(''); dialog('Add care relationship',`<p>Choose people in ${activeName()}.</p><div class="field"><label for="carer">Carer</label><select id="carer" name="carer">${options}</select></div><div class="field"><label for="person">Person receiving care</label><select id="person" name="person">${options}</select></div>`,'Add relationship', data => { if (data.get('carer') === data.get('person')) { flash('Choose two different people.'); return; } householdRelationships.get(activeHousehold).push(`${data.get('carer')} cares for ${data.get('person')}`); flash('Care relationship added in this prototype.'); render(); }); } if (action === 'remove-relationship') dialog('Remove care relationship', `<p>Remove ${escapeHtml(state.relationships[a])}? This changes the care access shown in this prototype.</p>`, 'Remove', () => { state.relationships.splice(Number(a),1); flash('Care relationship removed.'); render(); }); });
document.addEventListener('submit', e => {
  if (e.target.id === 'recovery-form') { e.preventDefault(); const person = document.querySelector('#recovery-person').value; const house = document.querySelector('#recovery-household').value; dialog('Review owner recovery', `<p>Make ${escapeHtml(person)} the owner of ${escapeHtml(house)}?</p><div class="callout">This simulated action represents a verified recovery. No account will be changed.</div>`, 'Confirm recovery', () => { state.recoveryResult = `${person} became owner of ${house}.`; flash('Owner recovery and audit event simulated.'); render(); }); }
  if (e.target.id === 'upload-form') { e.preventDefault(); const file = document.querySelector('#upload').files[0]; if (!file || !file.name.toLowerCase().endsWith('.zip')) { state.importError = 'Choose a .zip dictionary archive to continue.'; document.querySelector('#upload-error').textContent = state.importError; return; } state.importError = ''; state.importProgress = 0; render(); const timer = setInterval(() => { state.importProgress += 10; if (state.importProgress >= 100) { clearInterval(timer); state.importProgress = null; state.importHistory.unshift({date:'Just now', status:'Complete', detail:'148,203 entries imported · simulation'}); flash('Mock import complete. No file was uploaded.'); } if (state.section === 'NHS dm+d') render(); else if (state.section === 'Overview' && system()) {
      document.querySelector('#overview-import-status strong').textContent = importStatus();
      document.querySelector('#overview-import-date').textContent = `Last successful import: ${lastSuccessfulImport()?.date ?? 'None yet'}`;
    } }, 300); }
});
readLocation();
