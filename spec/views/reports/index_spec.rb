# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Reports::Index do
  subject(:report_view) do
    described_class.new(daily_data: daily_data, inventory_alerts: inventory_alerts)
  end

  let(:daily_data) do
    [
      { day_name: 'Mon', percentage: 100 },
      { day_name: 'Tue', percentage: 90 },
      { day_name: 'Wed', percentage: 80 }
    ]
  end

  let(:inventory_alerts) do
    [
      { medicine_name: 'Ibuprofen', days_left: 3, doses_left: 12 }
    ]
  end

  it 'renders the health report heading' do
    rendered = render report_view
    expect(rendered).to include('Health Report')
    expect(rendered).to include('Wellness Analytics')
  end

  it 'renders the compliance stats' do
    rendered = render report_view
    expect(rendered).to include('Overall Compliance')
    expect(rendered).to include('Total Doses Logged')
  end

  it 'renders the adherence timeline' do
    rendered = render report_view
    expect(rendered).to include('Adherence Timeline')
    expect(rendered).to include('Mon')
    expect(rendered).to include('Tue')
    expect(rendered).to include('Wed')
  end

  it 'renders the inventory alert with interpolated data' do
    rendered = render report_view
    expect(rendered).to include('Inventory Alert')
    expect(rendered).to include('Ibuprofen')
    expect(rendered).to include('exhausted in 3 days')
  end
end
