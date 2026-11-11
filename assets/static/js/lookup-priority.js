(() => {
  const list = document.querySelector('[data-lookup-priority]');
  if (!list) return;
  const input = document.querySelector('#medicine-lookup-source-priority');
  const status = document.querySelector('[data-lookup-status]');
  const rows = () => Array.from(list.children);
  const update = (moved) => {
    const items = rows();
    input.value = items.map(row => row.dataset.source).join(',');
    items.forEach((row, index) => {
      const rank = row.querySelector('[data-rank]');
      rank.textContent = index + 1;
      rank.setAttribute('aria-label', `Priority ${index + 1}`);
      row.querySelector('[data-priority-label]').textContent = index === 0 ? 'First choice' : `Fallback ${index}`;
      row.querySelector('[data-move="up"]').disabled = index === 0;
      row.querySelector('[data-move="down"]').disabled = index === items.length - 1;
    });
    if (moved) status.textContent = `${moved.dataset.name} moved to priority ${items.indexOf(moved) + 1} of ${items.length}.`;
  };
  const move = (row, direction) => {
    const items = rows();
    const index = items.indexOf(row);
    const other = items[index + direction];
    if (!other) return;
    if (direction < 0) list.insertBefore(row, other);
    else list.insertBefore(other, row);
    update(row);
  };
  list.addEventListener('click', event => {
    const button = event.target.closest('[data-move]');
    if (!button) return;
    const row = button.closest('[data-source]');
    move(row, button.dataset.move === 'up' ? -1 : 1);
    if (button.disabled) row.querySelector('.lookup-drag-handle').focus();
    else button.focus();
  });
  list.querySelectorAll('.lookup-drag-handle').forEach(handle => {
    handle.disabled = false;
    handle.addEventListener('keydown', event => {
      if (!['ArrowUp', 'ArrowDown'].includes(event.key)) return;
      event.preventDefault();
      move(handle.closest('[data-source]'), event.key === 'ArrowUp' ? -1 : 1);
      handle.focus();
    });
  });
  update();
  if (window.Sortable) new window.Sortable(list, {
    handle: '.lookup-drag-handle',
    draggable: '[data-source]',
    animation: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 180,
    ghostClass: 'lookup-sort-ghost',
    chosenClass: 'lookup-sort-chosen',
    onEnd: event => update(event.item),
  });
})();
