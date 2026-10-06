const relationship = document.querySelector('#invitation-form select[name="relationship_type"]');
const dependents = document.querySelector('[data-invitation-dependents]');
if (relationship && dependents) {
  const sync = () => {
    const enabled = ['parent', 'carer', 'family_member', 'professional'].includes(relationship.value);
    dependents.hidden = !enabled;
    dependents.disabled = !enabled;
  };
  relationship.addEventListener('change', sync);
  sync();
}
