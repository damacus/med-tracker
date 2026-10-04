window.LoomUI.init(document, {
  modalSelector: ".profile-dialog",
  triggerAttribute: "data-profile-dialog",
  closeAttribute: "data-profile-close",
  tabsSelector: ".profile-tabs",
  navigation: true,
  focusStorageKey: "profile-tab-focus",
  scrollLockClass: "profile-modal-open"
});

document.addEventListener("error", event => {
  if (event.target instanceof HTMLImageElement && event.target.matches("[data-profile-avatar-image]")) {
    event.target.remove();
  }
}, true);

const appearanceKey = "med-tracker-appearance";
const themeKey = "med-tracker-theme";
function syncAppearance() {
  const appearance = localStorage.getItem(appearanceKey) || "system";
  const theme = localStorage.getItem(themeKey) || "default";
  const dark = appearance === "dark" || (appearance === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  const root = document.documentElement;
  root.dataset.appearance = appearance;
  root.dataset.theme = theme;
  root.dataset.allowPalette = "true";
  for (const name of [...root.classList]) if (name.startsWith("theme-")) root.classList.remove(name);
  if (theme !== "default") root.classList.add(`theme-${theme}`);
  root.classList.toggle("dark", dark);
  root.style.colorScheme = dark ? "dark" : "light";
  const meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute("content", dark ? "#111827" : "#f8fafc");
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
