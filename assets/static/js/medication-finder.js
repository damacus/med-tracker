(() => {
  const root = document.querySelector('[data-medication-finder]');
  if (!root) return;
  const prefix = `/households/${encodeURIComponent(root.dataset.household)}/medications`;
  const form = root.querySelector('[data-barcode-form]');
  const medicineForm = root.querySelector('[data-medicine-form]');
  const input = form.elements.q;
  const status = root.querySelector('[data-lookup-status]');
  const results = root.querySelector('[data-lookup-results]');
  const manual = root.querySelector('[data-manual-entry]');
  const feedback = root.querySelector('[data-lookup-feedback]');
  const start = root.querySelector('[data-camera-start]');
  const stop = root.querySelector('[data-camera-stop]');
  const cameraStatus = root.querySelector('[data-camera-status]');
  const canAdjust = root.dataset.canAdjust === 'true';
  let request;
  let generation = 0;
  let camera;
  let cameraGeneration = 0;
  let audio;

  function prepareSound() {
    try {
      const AudioContext = window.AudioContext || window.webkitAudioContext;
      if (!AudioContext) return;
      if (!audio || audio.state === 'closed') audio = new AudioContext();
      audio.resume().catch(() => {});
    } catch {}
  }

  function scanSound() {
    try {
      if (!audio || audio.state === 'suspended') return;
      const oscillator = audio.createOscillator();
      const gain = audio.createGain();
      oscillator.frequency.setValueAtTime(880, audio.currentTime);
      gain.gain.setValueAtTime(0.12, audio.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, audio.currentTime + 0.12);
      oscillator.connect(gain);
      gain.connect(audio.destination);
      oscillator.start();
      oscillator.stop(audio.currentTime + 0.12);
    } catch {}
  }

  const link = (label, href, primary = false) => {
    const element = document.createElement('a');
    element.className = primary ? 'btn btn-primary' : 'btn btn-outline';
    element.textContent = label;
    element.href = href;
    return element;
  };

  async function find(mode = 'barcode') {
    const activeForm = mode === 'medicine' ? medicineForm : form;
    if (feedback) activeForm.after(feedback);
    const sequence = ++generation;
    request?.abort();
    request = new AbortController();
    results.replaceChildren();
    if (manual) manual.hidden = false;
    const query = mode === 'barcode' ? input.value.trim() : medicineForm.elements.q.value.trim();
    if (mode === 'barcode' && !/^\d{13,14}$/.test(query)) {
      status.textContent = 'This barcode cannot be used. Enter the 13- or 14-digit number printed on the pack, or search by medicine name.';
      return;
    }
    const params = new URLSearchParams({ q: query });
    if (mode === 'medicine') {
      if (medicineForm.elements.form.value) params.set('form', medicineForm.elements.form.value);
      if (medicineForm.elements.strength.value.trim()) params.set('strength', medicineForm.elements.strength.value.trim());
    }
    if (manual) manual.href = `${prefix}/new?${new URLSearchParams(mode === 'barcode' ? { barcode: query } : { name: query })}`;
    status.textContent = 'Searching medicines…';
    try {
      const response = await fetch(`${prefix}/lookup?${params}`, { signal: request.signal, headers: { Accept: 'application/json' } });
      if (response.redirected || response.status === 401) throw new Error('sign-in');
      if (response.status === 403) throw new Error('permission');
      if (!response.ok) throw new Error('lookup');
      const found = await response.json();
      if (sequence !== generation) return;
      const matches = found.matches || [];
      if (manual && mode === 'barcode' && matches.length) manual.hidden = true;
      if (matches.length || found.results.length) {
        form.querySelector('[type="submit"]').className = 'btn btn-outline';
      }
      status.textContent = matches.length > 1 ? 'Possible matches found in your household. Check the medicine and location before adding stock.' : matches.length === 1 ? 'Medicine found in your household. Check it is the same product before adding stock.' : found.results.length ? found.permissions.can_create ? 'Product found. Review the details before adding it.' : 'Product found, but only a household manager can add it. Ask a manager to add this medicine.' : manual ? 'No matching product. You can enter the medication manually.' : 'No matching product. Ask a household manager to add the medicine.';
      for (const match of matches) {
        const card = document.createElement('div');
        card.className = 'card card-body border border-base-300';
        const heading = document.createElement('h2');
        heading.className = 'card-title';
        heading.textContent = match.name;
        const detail = document.createElement('p');
        detail.textContent = `${match.location} · Current supply: ${match.current_supply || 'not tracked'}`;
        card.append(heading, detail);
        if (canAdjust) card.append(link('Add stock', `${prefix}/${match.id}?refill=true`, true));
        else {
          const note = document.createElement('p');
          note.textContent = 'Ask a household manager to add stock.';
          card.append(note);
        }
        results.append(card);
      }
      for (const product of mode === 'barcode' && matches.length ? [] : found.results) {
        const card = document.createElement('div');
        card.className = 'card card-body border border-base-300';
        const heading = document.createElement('h2');
        heading.className = 'card-title';
        heading.textContent = product.display;
        const source = document.createElement('p');
        source.textContent = product.source_label || '';
        card.append(heading, source);
        for (const [key, className] of [['description', 'text-base-content/80'], ['warnings', 'alert alert-warning']]) {
          if (!product[key]) continue;
          const paragraph = document.createElement('p');
          paragraph.className = className;
          paragraph.textContent = product[key];
          card.append(paragraph);
        }
        for (const related of product.related_medications || []) {
          card.append(link(`${related.name} · ${related.location} · ${related.current_supply || 'untracked'}`, related.path));
        }
        for (const prompt of product.review_prompts || []) {
          const note = document.createElement('p');
          note.className = 'alert alert-info';
          note.textContent = [prompt.risk_level_label, prompt.description, prompt.source_name].filter(Boolean).join(' · ');
          card.append(note);
        }
        if (found.review_guidance?.status === 'unavailable') {
          const note = document.createElement('p');
          note.textContent = 'Review guidance is temporarily unavailable.';
          card.append(note);
        }
        if (found.permissions.can_create) {
          const query = new URLSearchParams({ barcode: product.barcode || (mode === 'barcode' ? input.value : ''), dmd_code: product.code || '', dmd_concept_class: product.concept_class || '', dmd_system: product.system || '', name: product.display || '', description: product.description || '', warnings: product.warnings || '', category: product.category || '' });
          const firstDose = product.suggested_doses?.[0];
          if (firstDose) {
            query.set('dose_amount', String(firstDose.amount || ''));
            query.set('dose_unit', firstDose.unit || 'tablet');
            query.set('current_supply', String(firstDose.current_supply ?? ''));
            query.set('reorder_threshold', String(firstDose.reorder_threshold ?? '0'));
            query.set('suggested_doses', JSON.stringify(product.suggested_doses));
          }
          card.append(link('Add this medication', `${prefix}/new?${query}`, true));
        }
        results.append(card);
      }
    } catch (error) {
      if (error.name !== 'AbortError' && sequence === generation) status.textContent = error.message === 'sign-in' ? 'Your session has ended. Sign in again and retry the scan.' : error.message === 'permission' ? 'You no longer have access to search this household. Ask a household manager for access.' : 'Medication search is temporarily unavailable. Check your connection and try again.';
    }
  }

  async function stopCamera() {
    ++cameraGeneration;
    const active = camera;
    camera = undefined;
    start.disabled = false;
    stop.hidden = true;
    if (active) {
      try { await active.stop(); } catch {}
      try { active.clear(); } catch {}
    }
  }

  async function library() {
    if (window.__barcodeScannerTestLibrary) return window.__barcodeScannerTestLibrary;
    if (window.Html5Qrcode) return window;
    await new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = '/static/js/html5-qrcode.min.js';
      script.onload = resolve;
      script.onerror = reject;
      document.head.append(script);
    });
    return window;
  }

  start.addEventListener('click', async () => {
    prepareSound();
    const sequence = ++cameraGeneration;
    start.disabled = true;
    stop.hidden = false;
    stop.focus();
    cameraStatus.textContent = 'Requesting camera access…';
    let active;
    try {
      const { Html5Qrcode } = await library();
      const devices = await Html5Qrcode.getCameras();
      if (sequence !== cameraGeneration) return;
      if (!devices.length) throw new DOMException('No camera', 'NotFoundError');
      active = new Html5Qrcode('barcode-camera');
      camera = active;
      await active.start({ facingMode: 'environment' }, { fps: 10, qrbox: { width: 250, height: 150 } }, async value => {
        if (sequence !== cameraGeneration) return;
        scanSound();
        input.value = value;
        await stopCamera();
        cameraStatus.textContent = 'Barcode scanned.';
        await find();
        if (!root.closest('dialog') || root.closest('dialog').open) status.focus();
      }, () => {});
      if (sequence !== cameraGeneration) {
        try { await active.stop(); } catch {}
        try { active.clear(); } catch {}
        return;
      }
      cameraStatus.textContent = 'Point the camera at the pack barcode.';
    } catch (error) {
      if (sequence !== cameraGeneration) return;
      await stopCamera();
      if (cameraGeneration !== sequence + 1) return;
      cameraStatus.textContent = error.name === 'NotAllowedError' ? 'Camera permission was denied. Enter the barcode manually.' : error.name === 'NotFoundError' ? 'No camera is available. Enter the barcode manually.' : 'The camera could not start. Enter the barcode manually.';
      if (!root.closest('dialog') || root.closest('dialog').open) input.focus();
    }
  });
  stop.addEventListener('click', async () => {
    const sequence = cameraGeneration;
    await stopCamera();
    if (cameraGeneration !== sequence + 1 || (root.closest('dialog') && !root.closest('dialog').open)) return;
    cameraStatus.textContent = 'Scanner stopped.';
    start.focus();
  });
  form.addEventListener('submit', async event => { event.preventDefault(); await stopCamera(); await find(); });
  medicineForm?.addEventListener('submit', async event => { event.preventDefault(); await stopCamera(); await find('medicine'); });
  const dialog = root.closest('dialog');
  const closeScanner = () => {
    ++generation;
    request?.abort();
    stopCamera();
    cameraStatus.textContent = '';
  };
  dialog?.addEventListener('cancel', closeScanner);
  dialog?.addEventListener('close', closeScanner);
  dialog?.querySelector('[data-dialog-close]')?.addEventListener('click', closeScanner);
  window.addEventListener('pagehide', () => { request?.abort(); stopCamera(); audio?.close().catch(() => {}); });
  document.addEventListener('visibilitychange', () => { if (document.hidden) stopCamera(); });
  for (const control of root.querySelectorAll('[type="submit"], [data-camera-start]')) control.disabled = false;
})();
