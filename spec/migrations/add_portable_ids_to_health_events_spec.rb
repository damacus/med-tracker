# frozen_string_literal: true

require 'rails_helper'

load Rails.root.join('db/migrate/20260707110100_add_portable_ids_to_health_events.rb') unless
  defined?(AddPortableIdsToHealthEvents)

RSpec.describe AddPortableIdsToHealthEvents do
  delegate :connection, to: :'ActiveRecord::Base'

  it 'backfills populated health events through forced row-level security' do
    health_event = create_health_event
    clear_portable_id(health_event)

    with_owner_role { described_class.new.send(:prepare_portable_ids) }

    expect(portable_id_for(health_event)).to match(/\A[0-9a-f-]{36}\z/)
  end

  def create_health_event
    household = create(:household)
    HealthEvent.create!(
      household: household,
      person: create(:person, household: household),
      event_kind: :illness,
      title: 'Migration regression',
      started_on: Time.zone.today
    )
  end

  def clear_portable_id(health_event)
    connection.change_column_null(:health_events, :portable_id, true)
    connection.execute("UPDATE health_events SET portable_id = NULL WHERE id = #{connection.quote(health_event.id)}")
  end

  def portable_id_for(health_event)
    connection.select_value(
      "SELECT portable_id FROM health_events WHERE id = #{connection.quote(health_event.id)}"
    )
  end

  def with_owner_role
    connection.execute('SET LOCAL ROLE med_tracker_owner')
    yield
  ensure
    connection.execute('RESET ROLE')
  end
end
