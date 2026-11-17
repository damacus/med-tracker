document.addEventListener('click', event => {
  document.querySelectorAll('details[data-testid="dashboard-person-selector-disclosure"][open], details.dashboard-row-menu[open]').forEach(selector => {
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
  const disclosure = event.target.closest('details[data-testid="dashboard-person-selector-disclosure"], details.dashboard-row-menu');
  if (event.key === 'Escape' && disclosure?.open) {
    event.preventDefault();
    disclosure.open = false;
    disclosure.querySelector('summary').focus();
    return;
  }
  const selector = event.target.closest('details[data-testid="dashboard-person-selector-disclosure"]');
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
  const selector = event.target.closest('details[data-testid="dashboard-person-selector-disclosure"], details.dashboard-row-menu');
  if (selector?.open && !selector.contains(event.relatedTarget)) selector.open = false;
});

document.addEventListener('DOMContentLoaded', () => {
  const dashboard = document.querySelector('#dashboard');
  if (!dashboard) return;
  const desktop = window.matchMedia('(min-width: 1024px)');
  const panels = dashboard.querySelectorAll('[data-dashboard-responsive-panel]');
  const arrangePanels = () => panels.forEach(panel => {
    if (!desktop.matches && panel.contains(document.activeElement)) return;
    panel.open = desktop.matches;
  });
  arrangePanels();
  desktop.addEventListener('change', arrangePanels);

  const tray = dashboard.querySelector('[data-dashboard-next-action]');
  if (!tray) return;
  const dock = document.querySelector('[data-mobile-shortcuts]');
  const measure = () => {
    dashboard.style.setProperty('--dashboard-tray-height', `${Math.ceil(tray.getBoundingClientRect().height)}px`);
    dashboard.style.setProperty('--dashboard-dock-height', `${Math.ceil(dock?.getBoundingClientRect().height ?? 0)}px`);
  };
  measure();
  if (typeof ResizeObserver !== 'undefined') {
    const observer = new ResizeObserver(measure);
    observer.observe(tray);
    if (dock) observer.observe(dock);
  }
  window.addEventListener('resize', measure);
});
