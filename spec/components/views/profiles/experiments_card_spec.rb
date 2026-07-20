# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::ExperimentsCard, type: :component do
  fixtures :accounts

  let(:account) { accounts(:damacus) }

  it 'uses token-driven shells for experiments' do
    rendered = render_inline(described_class.new(account: account))
    html = rendered.to_html

    banned_classes = ['rounded-[2rem]', 'rounded-[1.2rem]', 'bg-card/95', 'bg-background/60',
                      'shadow-[0_18px_45px_-32px_rgba']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end

  it 'renders every dashboard design with the current dashboard selected by default', :aggregate_failures do
    rendered = render_inline(described_class.new(account: account))

    expect(rendered.text).to include('Dashboard layout')
    expect(rendered.text).to include('Current dashboard')
    expect(rendered.text).to include('Time-first')
    expect(rendered.text).to include('Family lanes')
    expect(rendered.text).to include('Calm focus')
    current_option = rendered.at_css('input[name="account[dashboard_variant]"][value="current"]')
    expect(current_option['checked']).to eq('checked')
  end
end
