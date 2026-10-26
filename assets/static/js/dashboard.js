document.addEventListener('click', event => {
  document.querySelectorAll('details[data-testid="dashboard-person-selector-disclosure"][open]').forEach(selector => {
    if (!selector.contains(event.target)) selector.open = false;
  });
  const option = event.target.closest('[data-person-id]');
  if (!option) return;
  const form = option.closest('form[data-dashboard-selector]');
  form.elements.dashboard_person_id.value = option.dataset.personId;
  form.requestSubmit();
});

document.addEventListener('keydown', event => {
  const selector = event.target.closest('details[data-testid="dashboard-person-selector-disclosure"]');
  if (event.key === 'Escape' && selector?.open) {
    event.preventDefault();
    selector.open = false;
    selector.querySelector('summary').focus();
    return;
  }
  const option = event.target.closest('[data-person-id]');
  if (!option || !['ArrowRight', 'ArrowLeft', 'ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  const options = [...option.closest('[role="radiogroup"]').querySelectorAll('[role="radio"]')];
  const index = options.indexOf(option);
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? options.length - 1 :
    (index + (['ArrowRight', 'ArrowDown'].includes(event.key) ? 1 : -1) + options.length) % options.length;
  options.forEach(button => { button.tabIndex = -1; });
  options[next].tabIndex = 0;
  options[next].focus();
});

document.addEventListener('focusout', event => {
  const selector = event.target.closest('details[data-testid="dashboard-person-selector-disclosure"]');
  if (selector?.open && !selector.contains(event.relatedTarget)) selector.open = false;
});
