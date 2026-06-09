# frozen_string_literal: true

class AuditActorResolver
  def initialize
    @cache = {}
  end

  def name_for(whodunnit)
    return I18n.t('admin.audit_logs.index.system') if whodunnit.blank?

    user = if @cache.key?(whodunnit)
             @cache[whodunnit]
           else
             @cache[whodunnit] = User.find_by(id: whodunnit)
           end
    user ? user.name : "User ##{whodunnit}"
  end
end
