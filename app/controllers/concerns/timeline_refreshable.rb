# frozen_string_literal: true

module TimelineRefreshable
  extend ActiveSupport::Concern

  private

  def build_timeline_streams_for(taken_source, take)
    medicine = taken_source.medicine.reload

    streams = [
      update_timeline_item_stream(taken_source, take),
      update_medicine_card_stream(taken_source)
    ]

    streams + other_timeline_streams(taken_source, medicine)
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

  def update_medicine_card_stream(source)
    if source.is_a?(Prescription)
      turbo_stream.replace(
        "prescription_#{source.id}",
        Components::Prescriptions::Card.new(prescription: source, person: source.person)
      )
    else
      turbo_stream.replace(
        "person_medicine_#{source.id}",
        Components::PersonMedicines::Card.new(person_medicine: source, person: source.person)
      )
    end
  end

  def other_timeline_streams(taken_source, medicine)
    streams = []

    other_prescriptions(taken_source, medicine).each do |p|
      stream = replace_timeline_item(p)
      streams << stream if stream
    end

    other_pms(taken_source, medicine).each do |pm|
      stream = replace_timeline_item(pm)
      streams << stream if stream
    end

    streams
  end

  def other_prescriptions(taken_source, medicine)
    scope = Prescription.where(active: true).where(medicine: medicine).includes(:person, :medicine, :dosage)
    taken_source.is_a?(Prescription) ? scope.where.not(id: taken_source.id) : scope
  end

  def other_pms(taken_source, medicine)
    scope = PersonMedicine.where(medicine: medicine).includes(:person, :medicine)
    taken_source.is_a?(PersonMedicine) ? scope.where.not(id: taken_source.id) : scope
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
