# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::ScheduleRow, type: :component do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  subject(:row) do
    described_class.new(
      person: person,
      schedule: schedule
    )
  end

  let(:person) { people(:john) }
  let(:schedule) { schedules(:active_schedule) }

  it 'renders the medication quantity' do
    rendered = render_inline(row)
    expect(rendered.text).to include(MedicationStockQuantityFormatter.format(schedule.medication.current_supply))
  end

  it 'renders the shared person avatar with initials instead of emoji' do
    rendered = render_inline(row)

    expect(rendered.text).not_to include('👤')
    expect(rendered.at_css('[data-testid="person-avatar"]')).to be_present
    expect(rendered.text).to include('JD')
  end

  it 'does not repeat the blocked-state stock lookup when a schedule is not blocked' do
    expect(count_stock_source_queries { render_inline(row) }).to eq(2)
  end

  def count_stock_source_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      sql = payload[:sql]
      count += 1 if sql.include?('FROM "medications"') &&
                    sql.include?('"medications"."name"') &&
                    sql.include?('"medications"."dosage_amount"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
