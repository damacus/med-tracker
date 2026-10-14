document.querySelectorAll('[data-inventory-filters]').forEach(form => {
  form.querySelectorAll('select').forEach(select => select.addEventListener('change', () => form.requestSubmit()));
});
