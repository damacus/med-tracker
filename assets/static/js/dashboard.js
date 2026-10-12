document.addEventListener('click', event => {
  const option = event.target.closest('[data-person-id]');
  if (!option) return;
  const form = option.closest('form[data-dashboard-selector]');
  form.elements.dashboard_person_id.value = option.dataset.personId;
  form.requestSubmit();
});

document.addEventListener('keydown', event => {
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
