# frozen_string_literal: true

class OfflineDoseEligibility
  Context = Data.define(:record_access, :resolver, :decision_context)

  def initialize(source:, user:, policy: nil, now: Time.current, context: nil)
    @source = source
    @user = user
    @now = now
    @record_access = context ? context.record_access : policy.take_medication?
    @resolver = context&.resolver
    @decision_context = context&.decision_context
  end

  def as_json(*)
    reason = blocked_reason
    {
      allowed: reason.nil?,
      reason: reason,
      valid_until: now.end_of_day.iso8601,
      dose_amount: dose_amount.nil? ? nil : BigDecimal(dose_amount.to_s).to_s('F'),
      dose_unit: dose_unit
    }
  end

  private

  attr_reader :source, :user, :now

  def blocked_reason
    return I18n.t('offline.record_access_required') unless @record_access

    resolver = @resolver || MedicationStockSourceResolver.new(source: source, user: user, taken_at: now)
    error = resolver.blocked_reason
    error ||= (@decision_context || MedicationDoseDecisionContext.new(source: source, taken_at: now)).blocked_reason
    return unless error

    key = error == :inactive ? :cooldown : error
    I18n.t("take_medications.#{key}", default: I18n.t('take_medications.failure'))
  end

  def dose_amount
    return source.effective_dose_amount(now.to_date) if source.respond_to?(:effective_dose_amount)

    source.default_dose_amount
  end

  def dose_unit
    return source.effective_dose_unit(now.to_date) if source.respond_to?(:effective_dose_unit)

    source.dose_unit
  end
end
