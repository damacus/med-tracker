# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::ListItemComponent, type: :component do
  fixtures :medications

  it 'renders the view link with a top-level frame target' do
    medication = medications(:paracetamol)
    medication_url = Rails.application.routes.url_helpers.medication_path(medication)

    rendered = render_inline(described_class.new(medication: medication))
    medication_link = rendered.css('a').find do |link|
      href = link['href'].to_s.split(/[?#]/).first
      href == medication_url
    end

    expect(medication_link).to be_present
    expect(medication_link['href']).to include(medication_url)
    expect(medication_link['data-turbo-frame']).to eq('_top')
  end
end
