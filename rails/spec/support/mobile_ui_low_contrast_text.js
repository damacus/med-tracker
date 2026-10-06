(() => {
  const parseColor = (value) => {
    const match = value.match(/rgba?\(([^)]+)\)/);
    if (!match) return null;
    const parts = match[1].split(/,\s*/).map(Number);
    return { r: parts[0], g: parts[1], b: parts[2], a: parts[3] ?? 1 };
  };

  const channel = (value) => {
    const normalized = value / 255;
    return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  };

  const luminance = (color) => 0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b);

  const contrast = (foreground, background) => {
    const lighter = Math.max(luminance(foreground), luminance(background));
    const darker = Math.min(luminance(foreground), luminance(background));
    return (lighter + 0.05) / (darker + 0.05);
  };

  const backgroundFor = (element) => {
    let current = element;
    while (current) {
      const color = parseColor(getComputedStyle(current).backgroundColor);
      if (color && color.a > 0.8) return color;
      current = current.parentElement;
    }
    return parseColor(getComputedStyle(document.body).backgroundColor);
  };

  const directText = (element) => Array.from(element.childNodes)
    .filter((node) => node.nodeType === Node.TEXT_NODE)
    .map((node) => node.textContent.trim())
    .filter(Boolean)
    .join(" ");

  return Array.from(document.querySelectorAll("body *"))
    .filter((element) => {
      if (["SCRIPT", "STYLE", "SVG", "PATH"].includes(element.tagName)) return false;
      if (element.closest("[aria-hidden=\"true\"], [hidden], [disabled], [aria-disabled=\"true\"]")) return false;
      const styles = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return styles.display !== "none" &&
        styles.visibility !== "hidden" &&
        Number(styles.opacity) >= 0.99 &&
        rect.width > 0 &&
        rect.height > 0 &&
        directText(element).length > 0;
    })
    .map((element) => {
      const styles = getComputedStyle(element);
      const foreground = parseColor(styles.color);
      const background = backgroundFor(element);
      if (!foreground || !background) return null;

      const fontSize = Number.parseFloat(styles.fontSize);
      const fontWeight = Number.parseInt(styles.fontWeight, 10) || 400;
      const minimum = fontSize >= 24 || (fontSize >= 18.66 && fontWeight >= 700) ? 3 : 4.5;
      const ratio = contrast(foreground, background);

      if (ratio >= minimum) return null;

      return {
        selector: element.id ? `#${element.id}` : element.tagName.toLowerCase(),
        role: element.getAttribute("role") || null,
        ratio: Math.round(ratio * 100) / 100,
        minimum,
        viewport: { width: window.innerWidth, height: window.innerHeight }
      };
    })
    .filter(Boolean)
    .slice(0, 5);
})()
