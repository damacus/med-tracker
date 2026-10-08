document.addEventListener('click', event => {
  const opener = event.target.closest('[data-dialog-open]');
  const closer = event.target.closest('[data-dialog-close]');
  if (opener) {
    const dialog = document.getElementById(opener.dataset.dialogOpen);
    if (!(dialog instanceof HTMLDialogElement)) return;
    dialog.showModal();
    dialog.addEventListener('close', () => opener.focus(), { once: true });
  }
  if (closer) closer.closest('dialog')?.close();
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
