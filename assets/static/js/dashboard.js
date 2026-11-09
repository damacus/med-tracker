document.addEventListener('click', event => {
  document.querySelectorAll('details[data-testid="dashboard-person-selector-disclosure"][open]').forEach(selector => {
    if (!selector.contains(event.target)) selector.open = false;
  });
  const option = event.target.closest('[data-person-id]');
  if (!option) return;
  const selector = option.closest("[data-dashboard-selector]");
  if (!selector || option.dataset.personId !== selector.dataset.selectedPerson) return;
  if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey || (event.button != null && event.button !== 0)) return;
  event.preventDefault();
  selector.open = false;
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
  if (!option || !selector || !['ArrowRight', 'ArrowLeft', 'ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  const options = [...selector.querySelectorAll('a[data-person-id]')];
  const index = options.indexOf(option);
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? options.length - 1 :
    (index + (['ArrowRight', 'ArrowDown'].includes(event.key) ? 1 : -1) + options.length) % options.length;
  options[next].focus();
});

document.addEventListener('focusout', event => {
  const selector = event.target.closest('details[data-testid="dashboard-person-selector-disclosure"]');
  if (selector?.open && !selector.contains(event.relatedTarget)) selector.open = false;
});
