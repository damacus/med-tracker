(() => {
  const trigger = document.querySelector('[data-open-administration]');
  const administration = document.querySelector('#administration-dialog');
  const dose = document.querySelector('#dose-dialog');
  if (!trigger || !administration || !dose) return;

  const close = () => {
    if (dose.open) dose.close();
    if (administration.open) administration.close();
    trigger.focus();
  };

  trigger.addEventListener('click', event => {
    event.preventDefault();
    administration.showModal();
  });

  for (const button of document.querySelectorAll('[data-open-dose]')) {
    button.addEventListener('click', () => {
      const fields = dose.querySelector('form');
      fields.elements.source_type.value = button.dataset.sourceKind;
      fields.elements.source_id.value = button.dataset.sourceId;
      fields.elements.dose_amount.value = button.dataset.doseAmount;
      fields.elements.dose_unit.value = button.dataset.doseUnit;
      fields.querySelector('button[type="submit"]').disabled = false;
      const eligible = button.dataset.stockIds.split(',');
      const select = fields.elements.taken_from_medication_id;
      const options = new Map(Array.from(select.options, option => [option.value, option]));
      for (const id of eligible) {
        const option = options.get(id);
        if (option) select.append(option);
      }
      for (const option of select.options) {
        const available = eligible.includes(option.value);
        option.hidden = !available;
        option.disabled = !available;
      }
      select.value = eligible[0];
      dose.querySelector('#dose-person').textContent = button.dataset.personName;
      dose.querySelector('#dose-display').textContent = button.dataset.sourceKind === 'schedule'
        ? 'Calculated for selected time'
        : `${button.dataset.doseAmount} ${button.dataset.doseUnit}`;
      administration.close();
      dose.showModal();
    });
  }

  for (const button of document.querySelectorAll('[data-close-dialog]')) {
    button.addEventListener('click', close);
  }
  dose.querySelector('[data-choose-source]')?.addEventListener('click', () => {
    dose.close();
    administration.showModal();
  });

  for (const dialog of [administration, dose]) {
    dialog.addEventListener('cancel', event => {
      event.preventDefault();
      close();
    });
  }
  if (dose.hasAttribute('data-reopen')) dose.showModal();
})();
