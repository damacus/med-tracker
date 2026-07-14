(() => {
  const viewport = {
    width: window.innerWidth,
    height: window.innerHeight
  };

  const visible = (element) => {
    if (!element) return false;
    const styles = window.getComputedStyle(element);
    const bounds = element.getBoundingClientRect();

    return styles.display !== 'none' &&
      styles.visibility !== 'hidden' &&
      Number(styles.opacity) > 0 &&
      bounds.width > 0 &&
      bounds.height > 0 &&
      !element.closest('[hidden], [aria-hidden="true"]');
  };

  const selectorFor = (element) => {
    if (element.id) return `#${element.id}`;
    const testid = element.getAttribute('data-testid');
    if (testid) return `[data-testid="${testid}"]`;
    return element.tagName.toLowerCase();
  };

  const roleFor = (element) => {
    if (element.getAttribute('role')) return element.getAttribute('role');
    if (element.tagName === 'A') return 'link';
    if (element.tagName === 'BUTTON') return 'button';
    if (element.tagName === 'INPUT') return element.type === 'checkbox' ? 'checkbox' : 'textbox';
    if (element.tagName === 'SELECT') return 'combobox';
    if (element.tagName === 'TEXTAREA') return 'textbox';
    return element.tagName.toLowerCase();
  };

  const accessibleName = (element) => {
    const ariaLabel = element.getAttribute('aria-label');
    if (ariaLabel && ariaLabel.trim()) return ariaLabel.trim();

    const labelledBy = element.getAttribute('aria-labelledby');
    if (labelledBy) {
      const labelledText = labelledBy.split(/\s+/)
        .map((id) => document.getElementById(id)?.textContent.trim())
        .filter(Boolean)
        .join(' ');
      if (labelledText) return labelledText;
    }

    if (element.labels?.length) {
      const labelText = Array.from(element.labels).map((label) => label.textContent.trim()).filter(Boolean).join(' ');
      if (labelText) return labelText;
    }

    const title = element.getAttribute('title');
    if (title && title.trim()) return title.trim();

    return element.textContent.trim();
  };

  const actionable = Array.from(document.querySelectorAll(
    'a[href], button, input, select, textarea, [role="button"], [role="link"], [tabindex]'
  )).filter((element) => visible(element) &&
    !element.classList.contains('sr-only') &&
    !element.disabled &&
    element.getAttribute('aria-disabled') !== 'true');

  const missingNames = actionable.filter((element) => !accessibleName(element)).map((element) => ({
    selector: selectorFor(element),
    role: roleFor(element),
    viewport
  }));

  const targetFailures = actionable.map((element) => {
    const target = element;
    const bounds = target.getBoundingClientRect();
    return {
      selector: selectorFor(element),
      role: roleFor(element),
      viewport,
      width: Math.round(bounds.width),
      height: Math.round(bounds.height)
    };
  }).filter((target) => target.width < 24 || target.height < 24);

  const focusFailures = [];
  actionable.forEach((element) => {
    element.focus({ preventScroll: true });
    const styles = window.getComputedStyle(element);
    const focusVisible = styles.outlineStyle !== 'none' ||
      styles.outlineWidth !== '0px' ||
      styles.boxShadow !== 'none';
    if (document.activeElement !== element || !focusVisible) {
      focusFailures.push({
        selector: selectorFor(element),
        role: roleFor(element),
        viewport,
        focusable: document.activeElement === element ? 1 : 0,
        indicator: focusVisible ? 1 : 0
      });
    }
  });

  const headingRoot = document.querySelector('main') || document.body;
  const headingLevels = Array.from(headingRoot.querySelectorAll('h1, h2, h3, h4, h5, h6'))
    .filter((heading) => {
      const styles = window.getComputedStyle(heading);
      return styles.display !== 'none' && styles.visibility !== 'hidden' &&
        !heading.hidden && heading.getAttribute('aria-hidden') !== 'true';
    })
    .map((heading) => Number(heading.tagName.slice(1)));
  const headingFailures = [];
  if (headingLevels.length > 0 && headingLevels[0] !== 1) {
    headingFailures.push({ selector: 'h1', role: 'heading', viewport, level: headingLevels[0], expected: 1, levels: headingLevels });
  }
  headingLevels.slice(1).forEach((level, index) => {
    const previous = headingLevels[index];
    if (level > previous + 1) {
      headingFailures.push({ selector: `h${level}`, role: 'heading', viewport, level, previous, levels: headingLevels });
    }
  });

  const pageWidth = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth);

  return {
    viewport,
    overflow: Math.max(0, Math.round(pageWidth - document.documentElement.clientWidth)),
    headings: {
      levels: headingLevels,
      failures: headingFailures
    },
    actionable: {
      count: actionable.length,
      missingNames,
      targetFailures,
      focusFailures
    }
  };
})()
