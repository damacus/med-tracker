# frozen_string_literal: true

require 'rails_helper'

module DatabasePoolOperations
end

RSpec.describe DatabasePoolOperations do
  let(:dashboard) do
    JSON.parse(Rails.root.join('config/observability/database_pool_dashboard.json').read)
  end
  let(:rules) do
    path = Rails.root.join('config/observability/database_pool_alerts.yml')
    YAML.safe_load_file(path).fetch('groups').first.fetch('rules')
  end

  it 'provides dashboard panels for capacity, utilization, waiting, and timeouts' do
    expressions = dashboard.fetch('panels').flat_map { |panel| panel.fetch('targets') }.pluck('expr').join(' ')

    expect(expressions).to include('medtracker_db_connection_pool_size')
    expect(expressions).to include('medtracker_db_connection_pool_in_use')
    expect(expressions).to include('medtracker_db_connection_pool_idle')
    expect(expressions).to include('medtracker_db_connection_pool_waiting')
    expect(expressions).to include('medtracker_db_connection_pool_timeouts_total')
  end

  it 'alerts only after sustained contention or a checkout timeout' do
    alerts = rules.index_by { |rule| rule.fetch('alert') }

    expect(alerts.fetch('MedTrackerDatabasePoolContention')).to include('for' => '5m')
    expect(alerts.fetch('MedTrackerDatabasePoolExhaustion')).to include('for' => '10m')
    expect(alerts.fetch('MedTrackerDatabasePoolCheckoutTimeouts')).to include('for' => '1m')
  end
end
