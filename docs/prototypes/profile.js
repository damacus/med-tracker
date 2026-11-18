const tabs = [...document.querySelectorAll('.profile-tabs a')];
const panels = [...document.querySelectorAll('.panel')];
const status = document.querySelector('#status');
const scenario = document.querySelector('#scenario');
const households = { single: ['Morgan household'], multiple: ['Morgan household', 'Taylor household'], none: [] };
const dialog = document.querySelector('#profile-dialog');
const reminderForm = document.querySelector('#notifications .household-content form');
const reminderControls = [...reminderForm.querySelectorAll('input')];
const reminderDefaults = reminderControls.map(input => input.type === 'checkbox' ? input.checked : input.value);
const reminderSettings = new Map();
let household = '';
let dialogAction = '';

function showTab() {
  const requested = location.hash.slice(1);
  const selected = panels.some(panel => panel.id === requested) ? requested : 'profile';
  panels.forEach(panel => { panel.hidden = panel.id !== selected; });
  tabs.forEach(tab => { if (tab.hash === `#${selected}`) tab.setAttribute('aria-current', 'page'); else tab.removeAttribute('aria-current'); });
  status.textContent = '';
}
function setHouseholds() {
  const choices = households[scenario.value];
  document.querySelectorAll('.household-content').forEach(element => { element.hidden = !choices.length; });
  document.querySelectorAll('.empty-household').forEach(element => { element.hidden = Boolean(choices.length); });
  document.querySelectorAll('.household-select').forEach(select => {
    select.replaceChildren(...choices.map(name => new Option(name, name)));
  });
  showHousehold(choices[0] || '');
}
function showHousehold(name) {
  household = name;
  document.querySelectorAll('.household-select').forEach(select => { select.value = name; });
  const badge = document.querySelector('#active-household');
  badge.textContent = name;
  badge.hidden = !name;
  selectHouseholdPalette();
  const values = reminderSettings.get(name) || reminderDefaults;
  reminderControls.forEach((input, index) => {
    if (input.type === 'checkbox') input.checked = values[index];
    else input.value = values[index];
  });
}
function flash(message) { status.textContent = message; }
window.addEventListener('hashchange', showTab);
tabs.forEach((tab, index) => {
  tab.addEventListener('click', event => { event.preventDefault(); history.replaceState(null, '', tab.hash); showTab(); });
  tab.addEventListener('keydown', event => {
    let next = index;
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
    else if (event.key === 'ArrowLeft') next = (index + tabs.length - 1) % tabs.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = tabs.length - 1;
    else return;
    event.preventDefault(); tabs[next].focus(); tabs[next].click();
  });
});
scenario.addEventListener('change', setHouseholds);
document.querySelectorAll('.household-select').forEach(select => select.addEventListener('change', () => {
  showHousehold(select.value);
}));
document.querySelectorAll('.local-form').forEach(form => form.addEventListener('submit', event => { event.preventDefault(); flash('Changes saved for this prototype session.'); }));
reminderForm.addEventListener('submit', () => {
  if (!household) return;
  reminderSettings.set(household, reminderControls.map(input => input.type === 'checkbox' ? input.checked : input.value));
  flash(`Reminder settings saved for ${household} in this prototype.`);
});
document.querySelectorAll('.save-button').forEach(button => button.addEventListener('click', () => flash('Changes saved for this prototype session.')));
const fullName = document.querySelector('#full-name');
const displayName = document.querySelector('#display-name');
const useDisplayName = document.querySelector('#use-display-name');
const displayNameField = document.querySelector('#display-name-field');

function updateNameField() {
  displayNameField.hidden = !useDisplayName.checked;
  displayName.disabled = !useDisplayName.checked;
  displayName.required = useDisplayName.checked;
}

useDisplayName.addEventListener('change', updateNameField);
updateNameField();
document.querySelector('#account-info').addEventListener('submit', () => {
  const name = (useDisplayName.checked ? displayName.value.trim() : '') || fullName.value.trim();
  document.querySelector('#profile-display-preview').textContent = name;
  const initials = name.split(/\s+/).slice(0, 2).map(part => Array.from(part)[0]).join('').toUpperCase();
  document.querySelectorAll('.profile-hero .avatar, .avatar-row .avatar').forEach(avatar => { avatar.textContent = initials; });
});

const appearance = document.querySelector('#appearance');
const palettes = [
  ['Command Centre', '#1b4fb8'],
  ['Serene Sage', '#7daa92'],
  ['Modern Clinical', '#0066ff'],
  ['Warm Earth', '#e07a5f'],
  ['Deep Lavender', '#9b5de5'],
  ['Forest Care', '#2d6a4f'],
  ['Sunset Support', '#f28482'],
  ['Tech Indigo', '#4361ee'],
  ['Soft Rose', '#e5989b'],
  ['Minty Fresh', '#06d6a0'],
];
const paletteOptions = document.querySelector('#palette-options');
const householdPalettes = new Map([
  ['Morgan household', palettes.findIndex(([label]) => label === 'Deep Lavender')],
  ['Taylor household', palettes.findIndex(([label]) => label === 'Serene Sage')],
]);
let personalPalette = 0;
try {
  const saved = JSON.parse(sessionStorage.getItem('medtracker-prototype-appearance'));
  if (saved && palettes[saved.palette]) personalPalette = saved.palette;
  if (saved && ['light', 'dark', 'system'].includes(saved.mode)) appearance.value = saved.mode;
  for (const name of householdPalettes.keys()) {
    const index = saved?.households?.[name];
    if (Number.isInteger(index) && palettes[index]) householdPalettes.set(name, index);
  }
} catch {
  // The preview also works when browser storage is unavailable.
}
for (const [index, [label, colour]] of palettes.entries()) {
  const option = document.createElement('label');
  option.className = 'palette-option';
  option.innerHTML = `<input type="radio" name="palette" value="${index}"><span class="palette-swatch" aria-hidden="true"></span><span>${label}</span>`;
  option.style.setProperty('--swatch', colour);
  paletteOptions.append(option);
}
function previewPalette(index) {
  if (index === 0) document.documentElement.style.removeProperty('--profile-palette');
  else document.documentElement.style.setProperty('--profile-palette', palettes[index][1]);
}
function selectHouseholdPalette() {
  const index = householdPalettes.get(household) ?? personalPalette;
  paletteOptions.querySelector(`input[value="${index}"]`).checked = true;
  previewPalette(index);
}
appearance.addEventListener('change', () => window.prototypeTheme?.(appearance.value));
paletteOptions.addEventListener('change', event => {
  previewPalette(Number(event.target.value));
});
appearance.form.addEventListener('submit', () => {
  const index = Number(paletteOptions.querySelector('input:checked').value);
  if (household) householdPalettes.set(household, index);
  else personalPalette = index;
  window.prototypeTheme?.(appearance.value);
  flash(household ? `Appearance saved for ${household}.` : 'Personal appearance saved.');
  try {
    sessionStorage.setItem('medtracker-prototype-appearance', JSON.stringify({
      mode: appearance.value,
      palette: personalPalette,
      households: Object.fromEntries(householdPalettes),
    }));
  } catch {
    // The current preview does not require storage.
  }
});
document.querySelector('#push-toggle').addEventListener('click', event => {
  const enabled = event.currentTarget.dataset.enabled !== 'true';
  event.currentTarget.dataset.enabled = String(enabled);
  event.currentTarget.textContent = enabled ? 'Disable on this device' : 'Enable on this device';
  document.querySelector('#push-state').textContent = enabled ? 'Enabled for this prototype' : 'Not enabled';
  document.querySelector('#push-test').disabled = !enabled;
});
document.querySelector('#push-test').addEventListener('click', () => flash('Test notification simulated. No notification was sent.'));
document.querySelector('#approve-support').addEventListener('click', event => { document.querySelector('#support-state').textContent = 'Access approved for Jamie Patel'; event.currentTarget.disabled = true; document.querySelector('#end-support').disabled = false; });
document.querySelector('#end-support').addEventListener('click', event => { document.querySelector('#support-state').textContent = 'Access ended'; event.currentTarget.disabled = true; document.querySelector('#approve-support').disabled = false; });
document.querySelector('#end-session').addEventListener('click', event => { event.currentTarget.closest('div').remove(); flash('Mobile session ended in this prototype.'); });
document.querySelector('#revoke-key').addEventListener('click', event => { event.currentTarget.closest('div').remove(); flash('API key revoked in this prototype.'); });
document.querySelectorAll('[data-export]').forEach(button => button.addEventListener('click', () => flash(`${button.dataset.export} export prepared for ${document.querySelector('#advanced .household-select').value}. No file is created in this prototype.`)));
const dialogContent = {
  history: ['Dose history', '<p>Your recorded doses are read-only.</p><div id="history-page"></div><div class="actions"><button type="button" id="history-prev">Previous</button><button type="button" id="history-next">Next</button></div>', 'Done'],
  plans: ['Care plans', '<p>Manage your personal care plans.</p><div class="setting-list"><div><span><strong>Everyday wellbeing</strong><small id="plan-state">Active</small></span><div class="actions"><button type="button" id="plan-toggle">Pause</button><button type="button" id="plan-retire">Retire</button></div></div></div>', 'Done'],
  guardian: ['Guardian declarations', '<p>Declarations record who can act on behalf of another person.</p><div class="notice"><strong>Sam Morgan</strong><p id="guardian-state">You declared that you are Sam’s guardian.</p><button type="button" id="guardian-withdraw">Withdraw declaration</button></div>', 'Done'],
  password: ['Change password', '<label class="field">Current password<input type="password" autocomplete="current-password"></label><label class="field">New password<input type="password" autocomplete="new-password"></label>', 'Update password'],
  passkey: ['Add a passkey', '<p>Passkeys let you sign in using your device. This prototype does not create a passkey.</p><label class="field">Passkey name<input placeholder="e.g. My laptop"></label>', 'Add passkey'],
  authenticator: ['Set up an authenticator', '<p>Use an authenticator app to add a second sign-in step.</p><label class="field">Verification code<input inputmode="numeric" placeholder="000000"></label>', 'Verify'],
  email: ['Change sign-in email', '<p>We would send a confirmation to your new address.</p><label class="field">New email<input type="email" placeholder="you@example.com"></label>', 'Continue'],
  provider: ['Connect a provider', '<p>Choose a sign-in provider to connect to your account.</p><label class="field">Provider<select><option>Google</option><option>Apple</option></select></label>', 'Continue'],
  recovery: ['Recovery codes', '<p>Example recovery codes. Store real codes securely and do not share them.</p><p><code>DEMO-4821 &nbsp; DEMO-9264</code></p>', 'Done'],
  'api-key': ['Create API key', '<p>This demonstration does not create a usable key.</p><label class="field">Key name<input placeholder="e.g. Home dashboard"></label>', 'Create key'],
  closure: ['Close your account', '<p>Account closure removes your access to your households and personal information. This demonstration will not close an account.</p><p class="notice" id="owner-warning"></p><label class="field">Type CLOSE to confirm<input id="close-confirm" autocomplete="off"></label>', 'Close account']
};
document.querySelectorAll('[data-dialog]').forEach(button => button.addEventListener('click', () => {
  dialogAction = button.dataset.dialog;
  const [title, content, action] = dialogContent[dialogAction];
  document.querySelector('#dialog-title').textContent = title;
  document.querySelector('#dialog-content').innerHTML = content;
  document.querySelector('#dialog-confirm').textContent = action;
  if (dialogAction === 'closure') document.querySelector('#owner-warning').textContent = scenario.value === 'none' ? 'You do not own a household in this scenario.' : 'You are the sole owner of Morgan household. Transfer ownership or close that household before closing your account.';
  if (dialogAction === 'history') {
    let page = 0;
    const render = () => { document.querySelector('#history-page').textContent = page === 0 ? '18 November 2026 · Vitamin D · Taken at 08:00' : '17 November 2026 · Vitamin D · Taken at 08:05'; document.querySelector('#history-prev').disabled = page === 0; document.querySelector('#history-next').disabled = page === 1; };
    document.querySelector('#history-prev').onclick = () => { page = 0; render(); };
    document.querySelector('#history-next').onclick = () => { page = 1; render(); };
    render();
  }
  if (dialogAction === 'plans') {
    document.querySelector('#plan-toggle').onclick = event => { const paused = event.currentTarget.textContent === 'Pause'; event.currentTarget.textContent = paused ? 'Resume' : 'Pause'; document.querySelector('#plan-state').textContent = paused ? 'Paused' : 'Active'; };
    document.querySelector('#plan-retire').onclick = event => { document.querySelector('#plan-state').textContent = 'Retired'; event.currentTarget.disabled = true; document.querySelector('#plan-toggle').disabled = true; };
  }
  if (dialogAction === 'guardian') document.querySelector('#guardian-withdraw').onclick = event => { document.querySelector('#guardian-state').textContent = 'Declaration withdrawn in this prototype.'; event.currentTarget.disabled = true; };
  dialog.showModal();
}));
dialog.addEventListener('close', () => {
  if (dialog.returnValue !== 'confirm') return;
  if (dialogAction === 'closure' && document.querySelector('#close-confirm').value !== 'CLOSE') { flash('Type CLOSE to confirm account closure. No account was closed.'); return; }
  if (dialogAction === 'closure' && scenario.value !== 'none') { flash('Transfer ownership or close Morgan household before closing your account.'); return; }
  flash(dialogAction === 'closure' ? 'Account closure demonstrated. No account was closed.' : 'Changes saved for this prototype session.');
});
showTab();
setHouseholds();
