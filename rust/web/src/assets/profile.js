document.addEventListener("click", event => {
  const trigger = event.target.closest("[data-profile-dialog]");
  if (trigger) {
    const dialog = document.getElementById(trigger.dataset.profileDialog);
    if (!dialog) return;
    dialog.showModal();
    dialog.addEventListener("close", () => trigger.focus(), { once: true });
    return;
  }
  const close = event.target.closest("[data-profile-close]");
  if (close) document.getElementById(close.dataset.profileClose)?.close();
});

const invalidDialog = document.querySelector(".profile-dialog[open]");
if (invalidDialog) {
  invalidDialog.removeAttribute("open");
  invalidDialog.showModal();
}
