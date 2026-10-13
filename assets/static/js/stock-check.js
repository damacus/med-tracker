(() => {
  const form = document.querySelector('[data-stock-check-form]');
  if (!form) return;
  const selections = [...document.querySelectorAll('[data-stock-check-select]')];
  const entries = new Map([...form.querySelectorAll('[data-stock-check-entry]')].map(entry => [entry.dataset.stockCheckEntry, entry]));
  const submit = form.querySelector('[data-stock-check-submit]');
  const empty = form.querySelector('[data-stock-check-empty]');
  const search = document.querySelector('[data-stock-check-search]');
  const selected = document.querySelector('[data-stock-check-selected]');
  const net = form.querySelector('[data-stock-check-net]');
  const number = new Intl.NumberFormat(document.documentElement.lang || 'en', { maximumFractionDigits: 2 });
  const signed = value => `${value > 0 ? '+' : ''}${number.format(value)}`;
  function refresh() {
    let count = 0;
    let invalid = false;
    const changes = new Map();
    for (const selection of selections) {
      const entry = entries.get(selection.value);
      if (!entry) continue;
      entry.classList.toggle('hidden', !selection.checked);
      for (const input of entry.querySelectorAll('input')) input.disabled = !selection.checked;
      if (!selection.checked) continue;
      count += 1;
      const original = entry.dataset.stockCheckCurrent;
      const quantity = entry.querySelector('input[type="number"]');
      const next = Number(quantity.value);
      const difference = entry.querySelector('[data-stock-check-difference]');
      if (quantity.value.trim() === '' || !quantity.validity.valid || !Number.isFinite(next)) {
        invalid = true;
        difference.textContent = '—';
      } else if (original === '') {
        difference.textContent = '—';
      } else {
        const delta = next - Number(original);
        difference.textContent = `${signed(delta)} ${entry.dataset.stockCheckUnit}`;
        changes.set(entry.dataset.stockCheckUnit, (changes.get(entry.dataset.stockCheckUnit) || 0) + delta);
      }
    }
    empty.hidden = count > 0;
    submit.disabled = count === 0 || invalid;
    submit.textContent = count === 1 ? submit.dataset.one : submit.dataset.other.replace('%{count}', count);
    selected.textContent = selected.dataset.template.replace('%{count}', count);
    net.textContent = changes.size ? [...changes].map(([unit, delta]) => `${signed(delta)} ${unit}`).join(' · ') : '—';
  }
  selections.forEach(selection => selection.addEventListener('change', refresh));
  for (const entry of entries.values()) {
    entry.querySelector('input[type="number"]').addEventListener('input', refresh);
    entry.querySelector('[data-stock-check-zero]').addEventListener('click', () => {
      entry.querySelector('input[type="number"]').value = '0';
      refresh();
    });
    entry.querySelector('[data-stock-check-remove]').addEventListener('click', () => {
      selections.find(selection => selection.value === entry.dataset.stockCheckEntry).checked = false;
      refresh();
    });
  }
  document.querySelectorAll('[data-stock-check-clear]').forEach(button => button.addEventListener('click', () => {
    selections.forEach(selection => { selection.checked = false; });
    refresh();
  }));
  search?.addEventListener('input', () => {
    const query = search.value.trim().toLocaleLowerCase();
    for (const row of document.querySelectorAll('[data-stock-check-name]')) {
      row.hidden = !row.dataset.stockCheckName.toLocaleLowerCase().includes(query);
    }
  });
  refresh();
})();
