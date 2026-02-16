# frozen_string_literal: true

module TimelineRefreshable
  extend ActiveSupport::Concern

  private

  def build_timeline_streams_for(taken_source, take)
    streams = []
    medicine = taken_source.medicine.reload

    streams << turbo_stream.replace(
      timeline_dom_id(taken_source),
      Components::Dashboard::TimelineItem.new(dose: {
                                                person: taken_source.person,
                                                source: taken_source,
                                                scheduled_at: take.taken_at,
                                                taken_at: take.taken_at,
                                                status: :taken
                                              })
    )

    other_prescriptions = Prescription.where(active: true)
                                      .where(medicine: medicine)
                                      .includes(:person, :medicine, :dosage)
    other_prescriptions = other_prescriptions.where.not(id: taken_source.id) if taken_source.is_a?(Prescription)

    other_prescriptions.each do |p|
      stream = replace_timeline_item(p)
      streams << stream if stream
    end

    other_pms = PersonMedicine.where(medicine: medicine)
                              .includes(:person, :medicine)
    other_pms = other_pms.where.not(id: taken_source.id) if taken_source.is_a?(PersonMedicine)

    other_pms.each do |pm|
      stream = replace_timeline_item(pm)
      streams << stream if stream
    end

    streams
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
