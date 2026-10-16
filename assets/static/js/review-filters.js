(() => {
  const toggle = document.querySelector('[data-review-hidden-toggle]');
  if (!toggle) return;
  toggle.addEventListener('change', () => toggle.form.requestSubmit());
  toggle.disabled = false;
})();
