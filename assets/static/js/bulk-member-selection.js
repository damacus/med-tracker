export function mountBulkMemberSelection(root) {
  const all = root.querySelector("[data-bulk-select-all]");
  const members = [...root.querySelectorAll('input[form="bulk-member-selection"][name="membership_ids"]')];
  if (!all) return;
  for (const member of members) member.checked = false;
  const controller = new AbortController();
  const sync = () => {
    const count = members.filter((member) => member.checked).length;
    all.checked = members.length > 0 && count === members.length;
    all.indeterminate = count > 0 && count < members.length;
  };
  all.addEventListener("change", () => {
    for (const member of members) member.checked = all.checked;
    sync();
  }, { signal: controller.signal });
  for (const member of members) member.addEventListener("change", sync, { signal: controller.signal });
  sync();
  return () => controller.abort();
}
