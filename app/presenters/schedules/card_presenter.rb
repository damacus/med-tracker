# frozen_string_literal: true

module Schedules
  class CardPresenter
    attr_reader :schedule, :todays_takes, :current_user, :person

    def initialize(schedule:, todays_takes:, current_user:, person:)
      @schedule = schedule
      @todays_takes = todays_takes
      @current_user = current_user
      @person = person
    end

    def status_top_border_class
      return 'border-t-rose-500' if out_of_stock?
      return 'border-t-amber-500' unless can_take_now?

      'border-t-primary'
    end

    def status_badge?
      !out_of_stock?
    end

    def status_badge_variant
      can_take_now? ? :tonal : :outlined
    end

    def status_badge_label
      return I18n.t('schedules.card.ready_now') if can_take_now?

      I18n.t('schedules.card.waiting')
    end

    def dose_description
      "#{dose_text} • #{schedule.frequency}"
    end

    def countdown_notice?
      !can_take_now? && schedule.countdown_display
    end

    def dose_count_badge?
      schedule.max_daily_doses.present?
    end

    def dose_count_label
      "#{todays_takes_count}/#{schedule.max_daily_doses}"
    end

    def resolved_todays_takes
      return @resolved_todays_takes if instance_variable_defined?(:@resolved_todays_takes)

      @resolved_todays_takes = (todays_takes || fetch_todays_takes).to_a
    end

    def todays_takes_count
      @todays_takes_count ||= resolved_todays_takes.size
    end

    def out_of_stock?
      blocked_reason == :out_of_stock
    end

    def can_take_now?
      return @can_take_now if instance_variable_defined?(:@can_take_now)

      @can_take_now = schedule.can_take_now?
    end

    def take_disabled?
      invalid_dose_configured? || out_of_stock?
    end

    def take_label
      own_dose? ? I18n.t('schedules.card.take') : I18n.t('schedules.card.give')
    end

    def take_state_label
      return I18n.t('schedules.card.invalid_dose') if invalid_dose_configured?
      return I18n.t('schedules.card.out_of_stock') if out_of_stock?

      take_label
    end

    private

    def dose_text
      "#{schedule.dose_amount.to_i}#{schedule.dose_unit}"
    end

    def fetch_todays_takes
      return loaded_todays_takes if schedule.medication_takes.loaded?

      queried_todays_takes
    end

    def loaded_todays_takes
      schedule.medication_takes
              .select { |take| take.taken_at >= Time.current.beginning_of_day }
              .sort_by(&:taken_at)
              .reverse
    end

    def queried_todays_takes
      schedule.medication_takes
              .where(taken_at: Time.current.beginning_of_day..)
              .order(taken_at: :desc)
    end

    def blocked_reason
      return @blocked_reason if instance_variable_defined?(:@blocked_reason)

      @blocked_reason = stock_source_resolver.blocked_reason
    end

    def invalid_dose_configured?
      schedule.dose_amount.to_f <= 0
    end

    def own_dose?
      return true if current_user.nil?

      current_user.person == person
    end

    def stock_source_resolver
      @stock_source_resolver ||= MedicationStockSourceResolver.new(user: current_user, source: schedule)
    end
  end
end
