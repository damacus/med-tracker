# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::ListItemComponent, type: :component do
  fixtures :locations, :medications

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

  it 'renders the edit link with a top-level frame target' do
    medication = medications(:paracetamol)
    edit_medication_url = Rails.application.routes.url_helpers.edit_medication_path(medication)

    rendered = render_inline(described_class.new(medication: medication))
    edit_medication_link = rendered.css('a').find do |link|
      href = link['href'].to_s.split(/[?#]/).first
      href == edit_medication_url
    end

    expect(edit_medication_link).to be_present
    expect(edit_medication_link['href']).to include(edit_medication_url)
    expect(edit_medication_link['data-turbo-frame']).to eq('_top')
  end

  it 'renders the friendly display name when present' do
    medication = medications(:paracetamol)
    medication.update!(
      name: 'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets',
      friendly_name: 'Movicol Paediatric Plain'
    )

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.at_css('h2').text).to include('Movicol Paediatric Plain')
    expect(rendered.at_css('h2').text).not_to include('Norgine Pharmaceuticals')
  end

  it 'renders each medicine with the medication icon' do
    medication = medications(:paracetamol)

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.at_css("div#medication_#{medication.id} svg.material-symbol-medication")).to be_present
  end

  it 'renders the medication icon inline with the medicine name' do
    medication = medications(:paracetamol)

    rendered = render_inline(described_class.new(medication: medication))
    header_group = rendered.at_css("div#medication_#{medication.id} div.flex.items-start.gap-3")

    expect(header_group.at_css('svg.material-symbol-medication')).to be_present
    expect(header_group.text).to include(medication.display_name)
  end

  it 'does not render the duplicate stock badge' do
    medication = medications(:paracetamol)

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).not_to include('In Stock')
  end
end
