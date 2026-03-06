# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card, type: :component do
  let(:person) { create(:person) }
  let(:medication) { create(:medication, name: 'Ibuprofen') }
  let(:dosage) { Dosage.create!(medication: medication, amount: 400.0, unit: 'mg', frequency: 'Twice daily') }
  let(:schedule) do
    Schedule.create!(
      person: person,
      medication: medication,
      dosage: dosage,
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  def render_card(todays_takes: nil)
    vc = view_context
    vc.singleton_class.define_method(:current_user) { nil }
    kwargs = { schedule: schedule, person: person }
    kwargs[:todays_takes] = todays_takes unless todays_takes.nil?
    html = vc.render(described_class.new(**kwargs))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def count_medication_take_queries(&)
    query_count = 0

    subscriber = lambda do |_name, _start, _finish, _id, payload|
      sql = payload[:sql]
      next if payload[:cached] || payload[:name] == 'SCHEMA'
      next unless sql.include?('"medication_takes"')

      query_count += 1
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    query_count
  end

  describe 'pre-loaded todays_takes' do
    let(:take) { MedicationTake.create!(schedule: schedule, taken_at: Time.current, amount_ml: 400) }

    it 'displays takes from pre-loaded collection' do
      rendered = render_card(todays_takes: [take])
      expect(rendered.text).to include('400mg')
    end

    it 'shows no doses message when pre-loaded takes is empty' do
      rendered = render_card(todays_takes: [])
      expect(rendered.text).to include('No doses taken today')
    end

    it 'falls back to querying when todays_takes is not provided' do
      take
      rendered = render_card
      expect(rendered.text).to include('400mg')
    end

    it 'reuses one medication_takes load for the dose badge and list' do
      schedule.update!(max_daily_doses: 4)
      allow(schedule).to receive_messages(out_of_stock?: false, can_take_now?: true)

      take

      expect(count_medication_take_queries { render_card }).to eq(1)
    end

    it 'memoizes schedule availability checks for the render' do
      resolver = instance_double(
        MedicationStockSourceResolver,
        blocked_reason: nil,
        available_medications: [medication]
      )
      allow(MedicationStockSourceResolver).to receive(:new).and_return(resolver)
      allow(schedule).to receive(:can_take_now?).and_return(true)

      render_card

      expect(schedule).to have_received(:can_take_now?).once
      expect(resolver).to have_received(:blocked_reason).once
    end
  end
end
