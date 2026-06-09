# frozen_string_literal: true

class AuditActorResolver
  def initialize
    @cache = {}
  end

  def name_for(whodunnit)
    return I18n.t('admin.audit_logs.index.system') if whodunnit.blank?

    user = (@cache[whodunnit] ||= User.find_by(id: whodunnit))
    user ? user.name : "User ##{whodunnit}"
  end
end
