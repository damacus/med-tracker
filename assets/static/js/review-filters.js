(() => {
  const toggle = document.querySelector('[data-review-hidden-toggle]');
  toggle?.addEventListener('change', () => toggle.form.requestSubmit());
})();
