function announceDoseReady(form, label) {
  const status = form.querySelector('[data-dose-availability]');
  const dose = document.createElement('span');
  dose.lang = 'en';
  dose.textContent = label;
  status.hidden = false;
  status.replaceChildren(document.createTextNode(`${status.dataset.readyLabel} `), dose);
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
    form.querySelector('[name="dose_amount"]').value = updatedDose.dataset.doseAmount;
    form.querySelector('[name="dose_unit"]').value = updatedDose.dataset.doseUnit;
    updatedDose.remove();
    form.querySelector('[data-dose-stale-alert]')?.setAttribute('hidden', '');
    announceDoseReady(form, form.querySelector('[data-dose-label]').textContent);
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
    form.querySelector('[name="dose_amount"]').value = preview.amount;
    form.querySelector('[name="dose_unit"]').value = preview.unit;
    form.querySelector('[data-dose-label]').textContent = preview.label;
    announceDoseReady(form, preview.label);
    submit.disabled = false;
  } catch {
    if (form.dataset.dosePreviewSequence !== String(sequence)) return;
    status.textContent = status.dataset.unavailableLabel;
  }
});

document.addEventListener('DOMContentLoaded', () => {
  for (const dialog of document.querySelectorAll('dialog[data-dialog-auto-open]')) {
    if (dialog instanceof HTMLDialogElement) {
      dialog.showModal();
      const opener = document.querySelector(`[data-dialog-open="${dialog.id}"]`);
      if (opener) dialog.addEventListener('close', () => opener.focus(), { once: true });
      dialog.querySelector('[data-dialog-error-focus]')?.focus();
    }
  }
});
