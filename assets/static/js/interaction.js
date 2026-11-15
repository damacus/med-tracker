function announceDoseReady(form) {
  const status = form.querySelector('[data-dose-availability]');
  status.hidden = false;
  status.textContent = status.dataset.readyLabel;
}

document.addEventListener('click', event => {
  const opener = event.target.closest('[data-dialog-open]');
  const closer = event.target.closest('[data-dialog-close]');
  const latest = event.target.closest('[data-refill-use-latest]');
  const updatedDose = event.target.closest('[data-dose-confirm-current]');
  if (latest) {
    latest.closest('dialog')?.querySelector('input[name="etag"]')?.setAttribute('value', latest.dataset.refillUseLatest);
    latest.disabled = true;
  }
  if (updatedDose) {
    const form = updatedDose.closest('form[data-dose-form]');
    const previous = form.querySelector('[name="expected_dose_amount"]').value;
    const actual = form.querySelector('[name="actual_dose_amount"]');
    const unitChanged = form.querySelector('[name="dose_unit"]').value !== updatedDose.dataset.doseUnit;
    if (unitChanged) actual.value = '';
    else if (actual.value === previous) actual.value = updatedDose.dataset.doseAmount;
    form.querySelector('[data-dose-unit-changed]').hidden = !unitChanged;
    form.querySelector('[data-actual-dose-unit]').textContent = updatedDose.dataset.doseUnit;
    form.querySelector('[name="dose_amount"]').value = updatedDose.dataset.doseAmount;
    form.querySelector('[name="expected_dose_amount"]').value = updatedDose.dataset.doseAmount;
    form.querySelector('[name="dose_unit"]').value = updatedDose.dataset.doseUnit;
    updatedDose.remove();
    form.querySelector('[data-dose-stale-alert]')?.setAttribute('hidden', '');
    announceDoseReady(form);
    const submit = form.querySelector('[type="submit"]');
    submit.disabled = form.dataset.doseCurrentAvailable !== 'true';
    if (!submit.disabled) submit.focus();
  }
  if (opener) {
    const dialog = document.getElementById(opener.dataset.dialogOpen);
    if (!(dialog instanceof HTMLDialogElement)) return;
    dialog.showModal();
    dialog.addEventListener('close', () => opener.focus(), { once: true });
  }
  if (closer) closer.closest('dialog')?.close();
});

document.addEventListener('change', async event => {
  const input = event.target.closest('form[data-dose-form] input[name="taken_at"]');
  if (!input) return;
  const form = input.form;
  const status = form.querySelector('[data-dose-availability]');
  const submit = form.querySelector('[type="submit"]');
  const updatedDose = form.querySelector('[data-dose-confirm-current]');
  const sequence = Number(form.dataset.dosePreviewSequence || 0) + 1;
  form.dataset.dosePreviewSequence = String(sequence);
  submit.disabled = true;
  if (updatedDose) updatedDose.disabled = true;
  form.querySelector('[data-dose-stale-alert]')?.setAttribute('hidden', '');
  status.hidden = false;
  status.textContent = status.dataset.checkingLabel;
  const params = new URLSearchParams({
    source_type: form.querySelector('[name="source_type"]').value,
    source_id: form.querySelector('[name="source_id"]').value,
    taken_at: input.value
  });
  try {
    const response = await fetch(`${form.dataset.dosePreviewUrl}?${params}`, { headers: { Accept: 'application/json' } });
    if (!response.ok) throw new Error('Dose preview unavailable');
    const preview = await response.json();
    if (form.dataset.dosePreviewSequence !== String(sequence)) return;
    if (!preview.available || !preview.amount || !preview.unit || !preview.label) {
      form.dataset.doseCurrentAvailable = 'false';
      updatedDose?.remove();
      status.textContent = status.dataset.notScheduledLabel;
      return;
    }
    form.dataset.doseCurrentAvailable = 'true';
    updatedDose?.remove();
    const previous = form.querySelector('[name="expected_dose_amount"]').value;
    const actual = form.querySelector('[name="actual_dose_amount"]');
    const unitChanged = form.querySelector('[name="dose_unit"]').value !== preview.unit;
    if (unitChanged) actual.value = '';
    else if (actual.value === previous) actual.value = preview.amount;
    form.querySelector('[data-dose-unit-changed]').hidden = !unitChanged;
    form.querySelector('[data-actual-dose-unit]').textContent = preview.unit;
    form.querySelector('[name="dose_amount"]').value = preview.amount;
    form.querySelector('[name="expected_dose_amount"]').value = preview.amount;
    form.querySelector('[name="dose_unit"]').value = preview.unit;
    form.querySelector('[data-dose-label]').textContent = preview.label;
    announceDoseReady(form);
    submit.disabled = false;
  } catch {
    if (form.dataset.dosePreviewSequence !== String(sequence)) return;
    status.textContent = status.dataset.unavailableLabel;
  }
});

document.addEventListener('DOMContentLoaded', () => {
  if (new URLSearchParams(window.location.search).get('refill') === 'true') {
    document.getElementById('refill-inventory-dialog')?.setAttribute('data-dialog-auto-open', '');
  }
  for (const dialog of document.querySelectorAll('dialog[data-dialog-auto-open]')) {
    if (dialog instanceof HTMLDialogElement) {
      dialog.showModal();
      let opener = document.querySelector(`[data-dialog-open="${dialog.id}"]`);
      const url = new URL(window.location.href);
      const recordingKeys = ['record_medication_id', 'record_source_type', 'record_source_id', 'record_person_id'];
      const dashboardRecording = dialog.id === 'record-dose-dialog' && url.searchParams.has('record_medication_id');
      if (dashboardRecording) {
        opener = Array.from(document.querySelectorAll('#dashboard a[href]')).find(link => {
          const destination = new URL(link.href);
          return destination.origin === url.origin && destination.pathname === url.pathname &&
            recordingKeys.every(key => url.searchParams.has(key) && destination.searchParams.get(key) === url.searchParams.get(key));
        });
      }
      dialog.addEventListener('close', () => {
        if (dashboardRecording) {
          const current = new URL(window.location.href);
          for (const key of recordingKeys) current.searchParams.delete(key);
          window.history.replaceState(window.history.state, '', current);
        }
        opener?.focus();
        if (dashboardRecording) opener?.scrollIntoView({ block: 'center' });
      }, { once: true });
      dialog.querySelector('[data-dialog-error-focus]')?.focus();
    }
  }
});
