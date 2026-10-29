(() => {
  const notice = document.querySelector("[data-canary-notice]");
  if (!notice) return;
  const dismissalKey = "med-tracker-canary-dismissed";
  try {
    if (sessionStorage.getItem(dismissalKey) === "true") {
      notice.hidden = true;
      return;
    }
  } catch {}
  let interval;
  notice.querySelector("[data-canary-dismiss]").addEventListener("click", () => {
    notice.hidden = true;
    window.clearInterval(interval);
    try { sessionStorage.setItem(dismissalKey, "true"); } catch {}
    document.getElementById("main")?.focus({preventScroll: true});
  });
  const last = notice.querySelector("[data-canary-last]");
  const next = notice.querySelector("[data-canary-next]");
  const countdown = notice.querySelector("[data-canary-countdown]");
  const formatter = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Europe/London", dateStyle: "medium", timeStyle: "long",
  });
  const dateLabel = (value, fallback) => {
    if (!value) return fallback;
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "Unavailable" : `${formatter.format(date)} (Europe/London)`;
  };
  const remaining = (value) => {
    const difference = Date.parse(value) - Date.now();
    if (!Number.isFinite(difference)) return "Reset details unavailable";
    if (difference <= 0) return "Reset due";
    const total = Math.ceil(difference / 60000);
    return [[Math.floor(total / 1440), "day"], [Math.floor(total / 60) % 24, "hour"], [total % 60, "minute"]]
      .filter(([amount]) => amount > 0).slice(0, 2)
      .map(([amount, unit]) => `${amount} ${unit}${amount === 1 ? "" : "s"}`).join(", ");
  };
  const refresh = async () => {
    if (notice.hidden) return;
    try {
      const response = await fetch("/canary/status", {cache: "no-store", credentials: "same-origin"});
      if (!response.ok) throw new Error("Status unavailable");
      const status = await response.json();
      last.textContent = dateLabel(status.last_success, "Not completed yet");
      next.textContent = dateLabel(status.next_reset, "Not scheduled");
      countdown.textContent = remaining(status.next_reset);
    } catch {
      last.textContent = "Unavailable";
      next.textContent = "Unavailable";
      countdown.textContent = "Reset details unavailable";
    }
  };
  refresh();
  interval = window.setInterval(refresh, 30000);
})();
