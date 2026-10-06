permit(principal, action == Action::"manage_household", resource)
when {
    principal.household_id == resource.household_id &&
    principal.status == "active" && !principal.revoked &&
    ["owner", "administrator"].contains(principal.role)
};

permit(principal, action in [Action::"view_person", Action::"record_person", Action::"manage_person"], resource)
when {
    principal.household_id == resource.household_id &&
    principal.status == "active" && !principal.revoked &&
    context.household_id == resource.household_id &&
    context.membership_id == principal.id && context.person_id == resource.id &&
    !context.revoked && (!context.expiring || context.expires_at > context.now) &&
    (context.level == "manage" ||
        (action == Action::"record_person" && context.level == "record") ||
        (action == Action::"view_person" && ["view", "record"].contains(context.level)))
};

permit(principal, action == Action::"delegate_person", resource)
when {
    principal.household_id == resource.household_id &&
    principal.status == "active" && !principal.revoked &&
    (["owner", "administrator"].contains(principal.role) ||
        ([1, 2].contains(context.person_type) && !context.has_capacity))
};

permit(principal, action == Action::"change_owner", resource)
when {
    principal.household_id == resource.household_id &&
    principal.status == "active" && !principal.revoked &&
    (context.platform_admin || (!context.next_owner && principal.role == "owner"))
};

permit(principal, action == Action::"support_household", resource)
when {
    context.trusted && context.platform_admin &&
    context.household_id == resource.household_id &&
    !context.ended && !context.expired && context.expires_at > context.now
};
