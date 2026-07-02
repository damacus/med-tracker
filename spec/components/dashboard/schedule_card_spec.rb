# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::ScheduleCard, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:person) { people(:john) }
  let(:schedule) { schedules(:active_schedule) }

  describe 'rendering' do
    it 'renders the person name' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(person.name)
    end

    it 'renders the medication name' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(schedule.medication.name)
    end

    it 'renders the schedule frequency' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(schedule.frequency)
    end

    it 'renders the medication quantity' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include(MedicationStockQuantityFormatter.format(schedule.medication.current_supply))
    end

    it 'renders the take action with a hand package icon' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))
      selector = "button[data-testid='take-medication-#{schedule.id}'] svg.material-symbol-hand-package"

      expect(rendered.css(selector)).to be_present
    end

    it 'does not repeat the blocked-state stock lookup when a schedule is not blocked' do
      expect(count_stock_source_queries do
        render_inline(described_class.new(person: person, schedule: schedule))
      end).to eq(2)
    end
  end

  describe 'card structure' do
    it 'renders with a schedule-specific id' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.css("#schedule_#{schedule.id}")).to be_present
    end

    it 'renders dosage details' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include('Dosage')
      expect(rendered.text).to include('Frequency')
    end

    it 'renders end date information' do
      rendered = render_inline(described_class.new(person: person, schedule: schedule))

      expect(rendered.text).to include('Ends')
    end
  end

  def count_stock_source_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      sql = payload[:sql]
      count += 1 if sql.include?('FROM "medications"') &&
                    sql.include?('"medications"."name"') &&
                    sql.include?('"medications"."dose_amount"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
