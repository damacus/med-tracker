(() => {
  const discardFailedImage = image => {
    if (image.matches?.('img[data-avatar-image]') && image.complete && image.naturalWidth === 0) image.remove();
  };
  document.addEventListener('error', event => discardFailedImage(event.target), true);
  new MutationObserver(records => {
    for (const record of records) for (const node of record.addedNodes) {
      discardFailedImage(node);
      node.querySelectorAll?.('img[data-avatar-image]').forEach(discardFailedImage);
    }
  }).observe(document.documentElement, { childList: true, subtree: true });
})();
