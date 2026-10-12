(() => {
  const root = document.querySelector('[data-medication-wizard]');
  if (!root) return;
  root.closest('dialog')?.addEventListener('close', () => {
    window.location.assign(root.querySelector('a.link').href);
  });
  const form = root.querySelector('form');
  const stages = [...root.querySelectorAll('[data-stage]')];
  const next = root.querySelector('[data-continue]');
  const back = root.querySelector('[data-back]');
  const save = root.querySelector('[data-save]');
  const field = name => form.elements[name];
  const value = name => field(name)?.value || '';
  const plan = name => root.querySelector('[data-plan-input="' + name + '"]')?.value || '';
  let stage = Math.min(Math.max(Number(value('wizard_step')) || 0, 0), stages.length - 1);
  let reviewed = false;
  let savedEndDate = value('end_date');
  function show() {
    stages.forEach((element, index) => { element.hidden = index !== stage; });
    root.querySelectorAll('.step').forEach((element, index) => {
      element.classList.toggle('step-primary', index <= stage);
      if (index === stage) element.setAttribute('aria-current', 'step');
      else element.removeAttribute('aria-current');
    });
    back.hidden = stage === 0;
    next.hidden = stage === stages.length - 1;
    save.hidden = stage !== stages.length - 1;
    field('wizard_step').value = String(stage);
    stages[stage].querySelector('h2').focus();
  }
  function shifted(time, hours) {
    const match = /^(\d{2}):(\d{2})$/.exec(time);
    if (!match) return '';
    const minutes = (Number(match[1]) * 60 + Number(match[2]) + hours * 60) % 1440;
    return String(Math.floor(minutes / 60)).padStart(2, '0') + ':' + String(minutes % 60).padStart(2, '0');
  }
  function schedule() {
    const kind = value('schedule_type');
    const count = Math.min(Math.max(Number(plan('count')) || 2, 1), 12);
    const hours = Number(plan('hours')) || 12;
    const first = plan('first');
    const second = plan('second');
    const timing = {
      multiple_daily: { frequency: count === 1 ? 'Once daily' : count === 2 ? 'Twice daily' : count === 3 ? 'Three times daily' : count + ' times daily', max: String(count), hours: String(hours), cycle: 'daily', times: count === 1 ? [first] : count === 2 ? [first, second] : Array.from({ length: count }, (_, index) => shifted(first, hours * index)) },
      daily: { frequency: 'Once daily', max: '1', hours: '24', cycle: 'daily', times: [plan('daily-time')] },
      weekly: { frequency: 'Once weekly', max: '1', hours: '168', cycle: 'weekly', times: [plan('weekly-time')] },
      specific_dates: { frequency: 'Specific dates', max: '1', hours: '24', cycle: 'daily', times: [] },
      prn: { frequency: 'As needed', max: plan('prn-max'), hours: plan('prn-hours'), cycle: 'daily', times: [] },
      tapering: { frequency: 'Tapering schedule', max: '1', hours: '24', cycle: 'daily', times: [] }
    }[kind];
    field('frequency').value = timing.frequency;
    field('max_daily_doses').value = timing.max;
    field('min_hours_between_doses').value = timing.hours;
    field('dose_cycle').value = timing.cycle;
    field('times').value = timing.times.filter(Boolean).join(', ');
    for (const day of ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday']) {
      field('weekday_' + day).checked = kind === 'weekly' && day === plan('weekly-day');
    }
    if (kind !== 'specific_dates') field('dates').value = '';
    if (kind === 'prn') {
      if (value('end_date')) savedEndDate = value('end_date');
      field('end_date').value = '';
    } else if (!value('end_date')) field('end_date').value = savedEndDate;
    root.querySelectorAll('[data-schedule-panel]').forEach(panel => { panel.hidden = panel.dataset.schedulePanel !== kind; });
    return timing;
  }
  function valid() {
    for (const input of stages[stage].querySelectorAll('input,select,textarea')) {
      if (input.type === 'hidden' || input.closest('[data-schedule-panel]')?.hidden) continue;
      if (!input.reportValidity()) return false;
    }
    if (stage === 1 && !reviewed) {
      root.querySelector('[data-plan-summary]').textContent = 'Review the medication plan before continuing.';
      root.querySelector('[data-review-plan]').focus();
      return false;
    }
    return true;
  }
  function readOptions(input) {
    try {
      const parsed = JSON.parse(input || '[]');
      return Array.isArray(parsed) ? parsed.slice(0, 10) : [];
    } catch {
      return [];
    }
  }
  let extras = value('extra_options') ? readOptions(value('extra_options')) : readOptions(value('suggested_doses')).slice(1);
  function persistOptions() {
    extras = [...root.querySelectorAll('[data-extra-option]')].map((fieldset, index) => ({
      ...extras[index],
      amount: fieldset.querySelector('[data-option-field="amount"]').value,
      unit: fieldset.querySelector('[data-option-field="unit"]').value,
      frequency: fieldset.querySelector('[data-option-field="frequency"]').value,
      current_supply: fieldset.querySelector('[data-option-field="current_supply"]').value,
      reorder_threshold: fieldset.querySelector('[data-option-field="reorder_threshold"]').value
    }));
    field('extra_options').value = JSON.stringify(extras);
  }
  function renderOptions() {
    const container = root.querySelector('[data-extra-options]');
    container.replaceChildren();
    extras.forEach((option, index) => {
      const group = document.createElement('fieldset');
      group.dataset.extraOption = String(index);
      group.className = 'grid gap-2 rounded-box border border-base-300 p-4';
      const legend = document.createElement('legend');
      legend.textContent = 'Dose option ' + (index + 2);
      group.append(legend);
      for (const [key, label, type] of [['amount', 'Amount', 'number'], ['unit', 'Unit', 'text'], ['frequency', 'Frequency', 'text'], ['current_supply', 'Starting supply', 'number'], ['reorder_threshold', 'Reorder threshold', 'number']]) {
        const id = 'extra-option-' + index + '-' + key;
        const caption = document.createElement('label');
        caption.htmlFor = id;
        caption.textContent = label;
        const input = document.createElement('input');
        input.id = id;
        input.dataset.optionField = key;
        input.type = type;
        input.className = 'input w-full';
        input.value = String(option[key] ?? '');
        if (type === 'number') { input.min = key === 'amount' ? '0.01' : '0'; input.step = '0.01'; }
        if (key === 'amount' || key === 'unit' || key === 'frequency') input.required = true;
        group.append(caption, input);
      }
      const remove = document.createElement('button');
      remove.type = 'button';
      remove.className = 'btn btn-ghost w-fit';
      remove.textContent = 'Remove dose option ' + (index + 2);
      remove.addEventListener('click', () => {
        persistOptions();
        extras.splice(index, 1);
        renderOptions();
        (container.querySelectorAll('[data-extra-option]')[Math.min(index, extras.length - 1)]?.querySelector('input') || root.querySelector('[data-add-option]')).focus();
      });
      group.append(remove);
      container.append(group);
    });
    field('extra_options').value = JSON.stringify(extras);
  }
  root.querySelector('[data-extra-options]').addEventListener('input', persistOptions);
  root.querySelector('[data-add-option]').addEventListener('click', () => {
    persistOptions();
    if (extras.length >= 10) return;
    extras.push({ amount: '', unit: value('dose_unit') || 'tablet', frequency: value('frequency') || 'As directed', default_max_daily_doses: Number(value('max_daily_doses')) || 1, default_min_hours_between_doses: Number(value('min_hours_between_doses')) || 0, default_dose_cycle: value('dose_cycle') || 'daily', current_supply: '', reorder_threshold: '0' });
    renderOptions();
    root.querySelector('[data-extra-option]:last-child input')?.focus();
  });
  renderOptions();
  next.addEventListener('click', () => { if (valid()) { stage++; show(); } });
  back.addEventListener('click', () => { stage--; show(); });
  stages[1].addEventListener('input', () => { reviewed = false; schedule(); });
  stages[1].addEventListener('change', () => { reviewed = false; schedule(); });
  root.querySelector('[data-review-plan]').addEventListener('click', () => {
    const timing = schedule();
    const person = field('person_id').selectedOptions[0]?.textContent || 'No person plan';
    const weekday = value('schedule_type') === 'weekly' ? plan('weekly-day') : '';
    root.querySelector('[data-plan-summary]').textContent = [value('dose_amount') + ' ' + value('dose_unit'), timing.frequency, weekday ? weekday[0].toUpperCase() + weekday.slice(1) : '', timing.times.join(', '), person, value('start_date') + (value('end_date') ? ' to ' + value('end_date') : '')].filter(Boolean).join(' · ');
    reviewed = true;
  });
  root.querySelector('[data-add-date]').addEventListener('click', () => {
    const date = plan('specific-date');
    if (!date) return;
    const dates = value('dates').split(',').map(item => item.trim()).filter(Boolean);
    if (!dates.includes(date)) dates.push(date);
    field('dates').value = dates.sort().join(', ');
    root.querySelector('[data-specific-dates-list]').textContent = dates.join(', ');
    root.querySelector('[data-plan-input="specific-date"]').value = '';
    reviewed = false;
  });
  form.addEventListener('submit', event => {
    schedule();
    persistOptions();
    if (stage !== 3 || !valid()) { event.preventDefault(); return; }
    for (const input of form.querySelectorAll('input,select,textarea')) {
      if (input.type === 'hidden' || input.closest('[data-schedule-panel]')?.hidden) continue;
      if (!input.checkValidity()) {
        event.preventDefault();
        stage = stages.findIndex(element => element.contains(input));
        show();
        input.reportValidity();
        return;
      }
    }
    save.disabled = true;
  });
  root.querySelector('[data-specific-dates-list]').textContent = value('dates');
  schedule();
  show();
})();
