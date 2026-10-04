function syncModalScroll() {
  document.documentElement.classList.toggle("profile-modal-open", Boolean(document.querySelector(".profile-dialog:modal")));
}

function openProfileDialog(dialog, trigger) {
  if (dialog.open) return;
  dialog.showModal();
  syncModalScroll();
  if (trigger) dialog.addEventListener("close", () => trigger.focus(), { once: true });
}

document.addEventListener("close", syncModalScroll, true);
document.addEventListener("click", event => {
  const dialog = event.target;
  if (!(dialog instanceof HTMLDialogElement) || !dialog.matches(".profile-dialog:modal")) return;
  const box = dialog.getBoundingClientRect();
  if (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom) dialog.close();
});

document.addEventListener("click", event => {
  const trigger = event.target.closest("[data-profile-dialog]");
  if (trigger) {
    const dialog = document.getElementById(trigger.dataset.profileDialog);
    if (!dialog) return;
    openProfileDialog(dialog, trigger);
    return;
  }
  const close = event.target.closest("[data-profile-close]");
  if (close) document.getElementById(close.dataset.profileClose)?.close();
});

document.addEventListener("error", event => {
  if (event.target instanceof HTMLImageElement && event.target.matches("[data-profile-avatar-image]")) {
    event.target.remove();
  }
}, true);

document.addEventListener("keydown", event => {
  const tab = event.target.closest('[role="tab"]');
  if (!tab || !tab.closest('[role="tablist"]')) return;
  const tabs = [...tab.parentElement.querySelectorAll('[role="tab"]')];
  const current = tabs.indexOf(tab);
  let next;
  if (event.key === "ArrowRight") next = tabs[(current + 1) % tabs.length];
  if (event.key === "ArrowLeft") next = tabs[(current + tabs.length - 1) % tabs.length];
  if (event.key === "Home") next = tabs[0];
  if (event.key === "End") next = tabs[tabs.length - 1];
  if (!next) return;
  event.preventDefault();
  sessionStorage.setItem("profile-tab-focus", next.id);
  window.location.assign(next.href);
});

const focusedTab = sessionStorage.getItem("profile-tab-focus");
if (focusedTab) {
  sessionStorage.removeItem("profile-tab-focus");
  const tab = document.getElementById(focusedTab);
  if (tab?.getAttribute("aria-selected") === "true") tab.focus();
}

const invalidDialog = document.querySelector(".profile-dialog[open]");
if (invalidDialog) {
  invalidDialog.removeAttribute("open");
  openProfileDialog(invalidDialog);
}

const appearanceKey = "med-tracker-appearance";
const themeKey = "med-tracker-theme";
const themeFonts = {
  default: "Plus Jakarta Sans", "serene-sage": "Inter", "modern-clinical": "Plus Jakarta Sans",
  "warm-earth": "Lexend", "deep-lavender": "Inter", "forest-care": "Outfit",
  "sunset-support": "Figtree", "tech-indigo": "Geist", "soft-rose": "Urbanist", "minty-fresh": "Public Sans"
};

function syncAppearance() {
  const appearance = localStorage.getItem(appearanceKey) || "system";
  const theme = localStorage.getItem(themeKey) || "default";
  const dark = appearance === "dark" || (appearance === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  const root = document.documentElement;
  root.dataset.appearance = appearance;
  root.dataset.theme = theme;
  root.classList.toggle("dark", dark);
  root.style.colorScheme = dark ? "dark" : "light";
  const meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute("content", dark ? "#111827" : "#f8fafc");
  root.style.setProperty("--font-family", `"${themeFonts[theme] || themeFonts.default}", sans-serif`);
  document.querySelectorAll("[data-appearance]").forEach(button => {
    button.setAttribute("aria-pressed", String(button.dataset.appearance === appearance));
  });
  document.querySelectorAll("[data-theme]").forEach(button => {
    button.setAttribute("aria-pressed", String(button.dataset.theme === theme));
  });
  document.querySelectorAll("[data-appearance-summary]").forEach(summary => {
    summary.textContent = appearance.charAt(0).toUpperCase() + appearance.slice(1);
  });
}

document.addEventListener("click", event => {
  const appearance = event.target.closest("[data-appearance]");
  const theme = event.target.closest("[data-theme]");
  if (appearance) localStorage.setItem(appearanceKey, appearance.dataset.appearance);
  if (theme) localStorage.setItem(themeKey, theme.dataset.theme);
  if (appearance || theme) syncAppearance();
});
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", syncAppearance);
syncAppearance();

document.addEventListener("submit", async event => {
  const upload = event.target.closest("[data-profile-avatar-upload]");
  const remove = event.target.closest("[data-profile-avatar-remove]");
  if (!upload && !remove) return;
  event.preventDefault();
  const form = upload || remove;
  const csrf = form.querySelector('[name="authenticity_token"]').value;
  const error = document.getElementById("profile-avatar-errors");
  const file = upload?.querySelector('[name="avatar"]').files[0];
  if (upload && !file) {
    error.textContent = "Choose a profile photo first.";
    error.hidden = false;
    return;
  }
  const body = new FormData();
  if (file) body.append("avatar", file);
  try {
    const response = await fetch(form.action, {
      method: upload ? "PUT" : "DELETE",
      credentials: "same-origin",
      headers: { "x-csrf-token": csrf },
      body: upload ? body : undefined
    });
    if (!response.ok) throw new Error("Profile photo could not be saved.");
    window.location.reload();
  } catch (failure) {
    error.textContent = failure.message;
    error.hidden = false;
  }
});
