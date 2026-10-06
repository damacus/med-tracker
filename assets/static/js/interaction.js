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
