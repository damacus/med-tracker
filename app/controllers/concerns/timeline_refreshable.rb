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
    turbo_stream.replace(
      timeline_dom_id(source),
      Components::Dashboard::TimelineItem.new(dose: {
                                                person: source.person,
                                                source: source,
                                                scheduled_at: take.taken_at,
                                                taken_at: take.taken_at,
                                                status: :taken
                                              })
    )
  end

  def update_medication_card_stream(source)
    if source.is_a?(Schedule)
      turbo_stream.replace(
        "schedule_#{source.id}",
        Components::Schedules::Card.new(schedule: source, person: source.person)
      )
    else
      turbo_stream.replace(
        "person_medication_#{source.id}",
        Components::PersonMedications::Card.new(person_medication: source, person: source.person)
      )
    end
  end

  def other_timeline_streams(taken_source, medication)
    streams = []

    other_schedules(taken_source, medication).each do |p|
      stream = replace_timeline_item(p)
      streams << stream if stream
    end

    other_pms(taken_source, medication).each do |pm|
      stream = replace_timeline_item(pm)
      streams << stream if stream
    end

    streams
  end

  def other_schedules(taken_source, medication)
    scope = Schedule.where(active: true).where(medication: medication).includes(:person, :medication, :dosage)
    taken_source.is_a?(Schedule) ? scope.where.not(id: taken_source.id) : scope
  end

  def other_pms(taken_source, medication)
    scope = PersonMedication.where(medication: medication).includes(:person, :medication)
    taken_source.is_a?(PersonMedication) ? scope.where.not(id: taken_source.id) : scope
  end

  def replace_timeline_item(source)
    next_time = source.next_available_time
    return nil unless next_time&.today?

    status = source.administration_blocked_reason || :upcoming

    turbo_stream.replace(
      timeline_dom_id(source),
      Components::Dashboard::TimelineItem.new(dose: {
                                                person: source.person,
                                                source: source,
                                                scheduled_at: next_time,
                                                taken_at: nil,
                                                status: status
                                              })
    )
  end

  def timeline_dom_id(source)
    "timeline_#{source.class.name.underscore}_#{source.id}"
  end
end
