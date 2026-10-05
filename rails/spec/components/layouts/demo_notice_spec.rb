# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::DemoNotice, type: :component do
  it 'identifies synthetic disposable data and the reset schedule' do
    rendered = render_inline(described_class.new)

    expect(rendered.css('[role="status"][aria-labelledby="demo-environment-title"]')).to be_present
    expect(rendered.text).to include('Demo environment')
    expect(rendered.text).to include('synthetic and disposable')
    expect(rendered.text).to include('Every Sunday at 04:15 Europe/London')
  end

  it 'lets the user dismiss the notice' do
    rendered = render_inline(described_class.new)

    notice = rendered.at_css('[data-controller="flash"]')
    button = notice.at_css('button[aria-label="Close"]')
    expect(notice['data-flash-dismiss-after-value']).to eq('0')
    expect(button['data-action']).to eq('click->flash#dismiss')
  end
end
