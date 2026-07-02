(() => {
  const viewportWidth = document.documentElement.clientWidth;

  return Array.from(document.querySelectorAll("body *"))
    .map((element) => {
      const rect = element.getBoundingClientRect();
      if (rect.right <= viewportWidth + 1 && rect.left >= -1) return null;

      return {
        tag: element.tagName.toLowerCase(),
        id: element.id || null,
        className: element.className.toString().slice(0, 160),
        text: element.textContent.trim().replace(/\s+/g, " ").slice(0, 80),
        left: Math.round(rect.left),
        right: Math.round(rect.right),
        width: Math.round(rect.width)
      };
    })
    .filter(Boolean)
    .slice(0, 5);
})()
