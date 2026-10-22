(() => {
  const all = document.querySelector("[data-bulk-select-all]");
  const members = [...document.querySelectorAll('input[form="bulk-member-selection"][name="membership_ids"]')];
  if (!all) return;
  const sync = () => {
    const count = members.filter((member) => member.checked).length;
    all.checked = count === members.length;
    all.indeterminate = count > 0 && count < members.length;
  };
  all.addEventListener("change", () => {
    for (const member of members) member.checked = all.checked;
    sync();
  });
  for (const member of members) member.addEventListener("change", sync);
  window.addEventListener("pageshow", () => {
    for (const member of members) member.checked = false;
    sync();
  });
})();
