# frozen_string_literal: true

module TimelineRefreshable
  extend ActiveSupport::Concern

  private

  def build_timeline_streams_for(taken_source, take)
    medication = taken_source.medication.reload

    streams = [
      update_timeline_item_stream(taken_source, take),
      update_medication_card_stream(taken_source)
    ]

    streams + other_timeline_streams(taken_source, medication)
  end

  def update_timeline_item_stream(source, take)
    if as_needed_source?(source)
      cooldown_stream = replace_timeline_item(source)
      return cooldown_stream if cooldown_stream
    end

    turbo_stream.replace(
      timeline_dom_id(source),
      Components::Dashboard::TimelineItem.new(dose: {
                                                person: source.person,
                                                source: source,
                                                scheduled_at: take.taken_at,
                                                taken_at: take.taken_at,
                                                status: :taken,
                                                taken_from_location_name: take.inventory_location&.name
                                              }, current_user: current_user)
    )
  end

  def update_medication_card_stream(source)
    if source.is_a?(Schedule)
      turbo_stream.replace(
        tenant_dom_id(source),
        Components::Schedules::Card.new(schedule: source, person: source.person, current_user: current_user)
      )
    else
      turbo_stream.replace(
        tenant_dom_id(source),
        Components::PersonMedications::Card.new(person_medication: source, person: source.person, current_user: current_user)
      )
    end
  end

  def other_timeline_streams(taken_source, medication)
    related_sources = MedicationTimelineQuery.new(
      medication: medication,
      excluding: taken_source,
      schedules_scope: policy_scope(Schedule),
      person_medications_scope: policy_scope(PersonMedication)
    ).call
    streams = []

    related_sources.schedules.each do |p|
      stream = replace_timeline_item(p)
      streams << stream if stream
    end

    related_sources.person_medications.each do |pm|
      stream = replace_timeline_item(pm)
      streams << stream if stream
    end

    streams
  end

  def replace_timeline_item(source)
    next_time = source.next_available_time
    return nil unless next_time&.today?

    status = MedicationStockSourceResolver.new(user: current_user, source: source).blocked_reason || :upcoming

    turbo_stream.replace(
      timeline_dom_id(source),
      Components::Dashboard::TimelineItem.new(dose: {
                                                person: source.person,
                                                source: source,
                                                scheduled_at: next_time,
                                                taken_at: nil,
                                                status: status
                                              }, current_user: current_user)
    )
  end

  def as_needed_source?(source)
    return source.as_needed? if source.respond_to?(:as_needed?)
    return false unless source.is_a?(Schedule)

    source.schedule_type_prn? ||
      source.schedule_config.to_h['as_needed'] == true ||
      source.frequency.to_s.casecmp('as needed').zero?
  end

  def timeline_dom_id(source)
    tenant_dom_target("timeline_#{source.class.name.underscore}_#{source.id}")
  end
end
