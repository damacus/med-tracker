# frozen_string_literal: true

module TenantDomTargetsHelper
  def tenant_dom_id(record, prefix = nil)
    tenant_dom_target(ActionView::RecordIdentifier.dom_id(record, prefix))
  end

  def tenant_dom_target(target)
    return target.to_s unless Current.household

    "household_#{Current.household.id}_#{target}"
  end
end
