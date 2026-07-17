# frozen_string_literal: true

module FamilyDashboard
  class StockStateLoader
    def initialize(sources:, person_ids:, current_user:, date:, now:)
      @sources = sources
      @person_ids = person_ids
      @current_user = current_user
      @date = date
      @now = now
    end

    def call
      preload_stock_source_candidates
      preload_recordable_person_ids
      self
    end

    def state_for(source)
      @states ||= {}
      @states[source] ||= build_state(source)
    end

    def daily_limit_reached?(source)
      timing_constraints_for(source).would_exceed_daily_limit?(
        takes: source.medication_takes.to_a,
        cycle: source_cycle(source),
        check_time: now
      )
    end

    def next_available_time_for(source)
      timing_constraints_for(source).next_available_time(
        takes: source.medication_takes.to_a,
        cycle: source_cycle(source),
        now: now
      )
    end

    private

    attr_reader :sources, :person_ids, :current_user, :date, :now

    def build_state(source)
      resolver = stock_source_resolver(source)
      blocked_reason = resolver.blocked_reason
      selection_required = blocked_reason.blank? && resolver.selection_required?(nil)
      status = selection_required ? :selection_required : blocked_reason

      {
        status: status,
        can_record: status.blank? && @recordable_person_ids.include?(source.person_id),
        choices: selection_required ? resolver.available_medications : []
      }
    end

    def stock_source_resolver(source)
      MedicationStockSourceResolver.new(
        user: current_user,
        source: source,
        taken_at: now,
        matching_medications: matching_stock_medications_for(source),
        can_take_at: can_take_source_at?(source)
      )
    end

    def preload_stock_source_candidates
      medications = scoped_stock_medications
      @stock_medications_by_signature = medications.group_by { |medication| medication_signature(medication) }
      @all_stock_medication_ids = medications.map(&:id)
      preload_assigned_medication_ids unless household_manager?
    end

    def scoped_stock_medications
      return sources.map(&:medication).uniq unless authorization_context

      MedicationPolicy::Scope.new(authorization_context, Medication.all)
                             .resolve
                             .includes(:location)
                             .to_a
    end

    def preload_recordable_person_ids
      @recordable_person_ids = if household_manager? || authorization_context.nil?
                                 person_ids
                               else
                                 recordable_person_ids
                               end
    end

    def recordable_person_ids
      PersonAccessGrant.active
                       .where(
                         household: authorization_context.household,
                         household_membership: authorization_context.membership,
                         person_id: person_ids,
                         access_level: %w[record manage]
                       )
                       .pluck(:person_id)
    end

    def preload_assigned_medication_ids
      @assigned_medication_ids_by_person = Hash.new { |hash, key| hash[key] = [] }
      return unless authorization_context

      assigned_medication_pairs.each do |person_id, medication_id|
        @assigned_medication_ids_by_person[person_id] << medication_id
      end
    end

    def assigned_medication_pairs
      schedules = Schedule.where(household: authorization_context.household, person_id: person_ids)
                          .pluck(:person_id, :medication_id)
      direct = PersonMedication.where(household: authorization_context.household, person_id: person_ids)
                               .pluck(:person_id, :medication_id)
      schedules + direct
    end

    def matching_stock_medications_for(source)
      candidates = @stock_medications_by_signature.fetch(medication_signature(source.medication), [])
      candidates = candidates.select { |medication| allowed_stock_medication_ids(source).include?(medication.id) }
      candidates.sort_by { |medication| [medication.location&.name.to_s, medication.id] }
    end

    def allowed_stock_medication_ids(source)
      return @all_stock_medication_ids if household_manager?
      return [source.medication_id] unless authorization_context

      (@assigned_medication_ids_by_person.fetch(source.person_id, []) + [source.medication_id]).uniq
    end

    def medication_signature(medication)
      [medication.name, medication.dose_amount.to_s, medication.dose_unit]
    end

    def authorization_context
      @authorization_context ||= resolve_authorization_context || false
      @authorization_context || nil
    end

    def resolve_authorization_context
      return current_user if current_user.is_a?(AuthorizationContext)

      AuthorizationContext.current || derived_authorization_context
    end

    def derived_authorization_context
      return unless current_user.respond_to?(:person)

      account = current_user.person&.account
      membership = account&.first_active_household_membership
      return unless membership

      AuthorizationContext.new(account: account, household: membership.household, membership: membership)
    end

    def household_manager?
      active_household_manager? || active_support_manager?
    end

    def active_household_manager?
      membership = authorization_context&.membership
      membership&.active? && (membership.owner? || membership.administrator?)
    end

    def active_support_manager?
      context = authorization_context
      return false unless context

      platform_admin = context.account&.platform_admin
      support_session = Current.support_access_session
      platform_admin&.active? && support_session&.active? && support_session.household_id == context.household.id
    end

    def can_take_source_at?(source)
      timing_constraints_for(source).satisfied_by?(
        takes: source.medication_takes.to_a,
        cycle: source_cycle(source),
        check_time: now
      )
    end

    def timing_constraints_for(source)
      @timing_constraints ||= {}
      @timing_constraints[source] ||= DoseConstraints.new(
        max_daily_doses: max_daily_doses_for(source),
        min_hours_between_doses: min_hours_between_doses_for(source)
      )
    end

    def max_daily_doses_for(source)
      source.is_a?(Schedule) ? source.effective_max_daily_doses(date) : source.max_daily_doses
    end

    def min_hours_between_doses_for(source)
      source.is_a?(Schedule) ? source.effective_min_hours_between_doses(date) : source.min_hours_between_doses
    end

    def source_cycle(source)
      DoseCycle.new(source.respond_to?(:dose_cycle) ? source.dose_cycle : 'daily')
    end
  end
end
