(() => {
  const panel = document.querySelector('[data-global-search]');
  if (!panel) return;

  const input = panel.querySelector('#global_search_query');
  const results = panel.querySelector('[data-global-search-results]');
  const status = panel.querySelector('[data-global-search-status]');
  const form = panel.querySelector('[data-global-search-form]');
  const triggers = [...document.querySelectorAll('[data-global-search-trigger]')];
  let opener;
  let active = -1;
  let controller;
  let timer;
  let generation = 0;

  const labels = panel.dataset;

  if (panel.dataset.labelsReady === 'false') {
    fetch(`/households/${panel.dataset.household}/search/labels.json`, { headers: { Accept: 'application/json' } })
      .then(response => response.ok ? response.json() : Promise.reject(new Error('Labels unavailable')))
      .then(({ locale, labels: translated }) => {
        panel.lang = locale;
        triggers.forEach(button => {
          button.lang = locale;
          button.setAttribute('aria-label', translated.open);
          button.textContent = translated.open_short;
        });
        panel.setAttribute('aria-label', translated.dialog_label);
        panel.querySelector('label[for="global_search_query"]').textContent = translated.input_label;
        input.placeholder = translated.placeholder;
        panel.querySelector('[data-global-search-close]').setAttribute('aria-label', translated.close);
        Object.assign(panel.dataset, {
          loading: translated.loading,
          noResults: translated.no_results,
          error: translated.error,
          retry: translated.retry,
          resultOne: translated.results.one,
          resultOther: translated.results.other,
          typeCommand: translated.types.command,
          typePerson: translated.types.person,
          typeMedication: translated.types.medication,
          typePersonMedication: translated.types.person_medication,
          typeSchedule: translated.types.schedule,
          typeLocation: translated.types.location,
          labelsReady: 'true'
        });
      })
      .catch(() => {});
  }

  function rows() { return [...results.querySelectorAll('[data-global-search-result]')]; }

  function setActive(index) {
    const links = rows();
    if (!links.length) return;
    active = (index + links.length) % links.length;
    links.forEach((link, position) => {
      const selected = position === active;
      link.dataset.globalSearchActive = selected ? 'true' : 'false';
      link.classList.toggle('border-primary', selected);
      link.classList.toggle('bg-base-200', selected);
    });
    links[active].scrollIntoView({ block: 'nearest' });
    status.textContent = links[active].textContent.trim();
  }

  function clear() {
    active = -1;
    results.replaceChildren();
    status.textContent = '';
  }

  function abort() {
    generation += 1;
    controller?.abort();
    controller = undefined;
    clearTimeout(timer);
  }

  function position() {
    panel.style.left = '50%';
    panel.style.top = '50%';
    panel.style.width = 'min(40rem, calc(100vw - 2rem))';
    panel.style.transform = 'translate(-50%, -50%)';
  }

  function open(event) {
    event?.preventDefault();
    if (!panel.hidden) {
      input.focus({ preventScroll: true });
      return;
    }
    const clickedTrigger = event?.currentTarget instanceof Element ? event.currentTarget : undefined;
    opener = clickedTrigger || document.activeElement;
    position();
    panel.hidden = false;
    panel.setAttribute('aria-hidden', 'false');
    triggers.forEach(button => button.setAttribute('aria-expanded', 'true'));
    input.value = '';
    clear();
    requestAnimationFrame(() => input.focus({ preventScroll: true }));
  }

  function close(event, restoreFocus = true) {
    event?.preventDefault();
    if (panel.hidden) return;
    abort();
    panel.hidden = true;
    panel.setAttribute('aria-hidden', 'true');
    triggers.forEach(button => button.setAttribute('aria-expanded', 'false'));
    if (restoreFocus) opener?.focus?.({ preventScroll: true });
  }

  function addResult(result, group) {
    if (typeof result.path !== 'string' || !result.path.startsWith(`/households/${panel.dataset.household}/`) || result.path.startsWith('//')) return;
    const link = document.createElement('a');
    link.href = result.path;
    link.className = 'block rounded-lg border border-transparent p-3 hover:border-primary hover:bg-base-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary';
    link.dataset.globalSearchResult = '';
    link.dataset.globalSearchActive = 'false';
    const title = document.createElement('strong');
    title.className = 'block';
    title.textContent = result.title || '';
    const subtitle = document.createElement('span');
    subtitle.className = 'block text-sm';
    subtitle.textContent = result.subtitle || '';
    link.append(title, subtitle);
    link.addEventListener('mouseenter', () => setActive(rows().indexOf(link)));
    group.append(link);
  }

  function addResults(items) {
    const groups = new Map();
    for (const item of items) {
      const type = item.type;
      const name = {
        command: labels.typeCommand,
        person: labels.typePerson,
        medication: labels.typeMedication,
        person_medication: labels.typePersonMedication,
        schedule: labels.typeSchedule,
        location: labels.typeLocation
      }[type];
      if (!name) continue;
      if (!groups.has(type)) {
        const section = document.createElement('section');
        const heading = document.createElement('h2');
        heading.className = 'px-3 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70';
        heading.textContent = name;
        section.append(heading);
        results.append(section);
        groups.set(type, section);
      }
      addResult(item, groups.get(type));
    }
  }

  async function search(query) {
    abort();
    const sequence = generation;
    if (!query) { clear(); return; }
    results.replaceChildren();
    status.textContent = labels.loading;
    controller = new AbortController();
    const url = new URL(panel.dataset.searchUrl, location.origin);
    url.searchParams.set('q', query);
    try {
      const response = await fetch(url, { headers: { Accept: 'application/json' }, signal: controller.signal });
      if (!response.ok) throw new Error('Search unavailable');
      const data = await response.json();
      if (!Array.isArray(data.results)) throw new Error('Invalid search response');
      if (generation !== sequence) return;
      clear();
      addResults(data.results);
      status.textContent = rows().length ? (rows().length === 1 ? labels.resultOne : labels.resultOther).replace('%{count}', rows().length) : labels.noResults;
      if (!rows().length) results.textContent = labels.noResults;
    } catch (error) {
      if (error.name === 'AbortError' || generation !== sequence) return;
      clear();
      const message = document.createElement('p');
      message.textContent = labels.error;
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'btn btn-outline mt-2';
      retry.textContent = labels.retry;
      retry.addEventListener('click', () => { search(input.value.trim()); input.focus(); });
      results.append(message, retry);
      status.textContent = labels.error;
    }
  }

  triggers.forEach(button => button.addEventListener('click', open));
  panel.querySelector('[data-global-search-close]').addEventListener('click', close);
  input.addEventListener('input', () => {
    abort();
    clear();
    timer = setTimeout(() => search(input.value.trim()), 150);
  });
  input.addEventListener('keydown', event => {
    if (event.key === 'ArrowDown') { event.preventDefault(); setActive(active + 1); }
    else if (event.key === 'ArrowUp') { event.preventDefault(); setActive(active - 1); }
    else if (event.key === 'Enter' && rows().length) {
      event.preventDefault();
      location.assign((rows()[active] || rows()[0]).href);
    }
  });
  panel.addEventListener('keydown', event => {
    if (event.key === 'Escape') close(event);
  });
  form.addEventListener('submit', event => {
    if (!rows().length) return;
    event.preventDefault();
    location.assign((rows()[active] || rows()[0]).href);
  });
  document.addEventListener('keydown', event => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') open(event);
  });
  document.addEventListener('pointerdown', event => {
    if (!panel.hidden && !panel.contains(event.target) && !triggers.some(button => button.contains(event.target))) close();
  });
  document.addEventListener('focusin', event => {
    if (!panel.hidden && !panel.contains(event.target)) close(undefined, false);
  });
})();
