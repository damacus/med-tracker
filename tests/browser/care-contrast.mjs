export async function measureCareContrast(page) {
  await page.waitForFunction(() => !document.getAnimations().some(animation =>
    animation instanceof CSSTransition && animation.playState === 'running'
  ), undefined, { timeout: 5000 });
  return page.evaluate(() => {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 1;
    const context = canvas.getContext('2d');
    const channels = color => {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return [...context.getImageData(0, 0, 1, 1).data];
    };
    const luminance = values => values.slice(0, 3).map(value => {
      const channel = value / 255;
      return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
    }).reduce((sum, channel, index) => sum + channel * [0.2126, 0.7152, 0.0722][index], 0);
    return ['main h1', 'main section > p', '.card p', '.card label', '.card input', '.card select', '.card textarea', '.btn-neutral', '.card .btn-ghost', '[role="alert"]', '.shell-muted'].flatMap(selector => {
      const element = document.querySelector(selector);
      if (!element) return [];
      const foreground = channels(getComputedStyle(element).color);
      let parent = element;
      let background = [255, 255, 255, 255];
      while (parent) {
        const color = channels(getComputedStyle(parent).backgroundColor);
        if (color[3] === 255) { background = color; break; }
        parent = parent.parentElement;
      }
      const alpha = foreground[3] / 255;
      const painted = foreground.map((channel, index) => index < 3 ? channel * alpha + background[index] * (1 - alpha) : 255);
      const first = luminance(painted);
      const second = luminance(background);
      return [{ selector, foreground: painted, background, ratio: (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05) }];
    });
  });
}
