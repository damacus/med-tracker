# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Locations::ShowView, type: :component do
  let(:location) { create(:location, name: 'Overflow Test Location') }
  let(:medication_name) { 'Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)' }
  let!(:medication) do
    create(:medication, location: location, name: medication_name, current_supply: 12, reorder_threshold: 10)
  end

  it 'keeps long medication names and supply badges within the medication card layout' do
    rendered = render_location

    name_link = rendered.at_css("a[href='/medications/#{medication.id}']")
    expect(name_link.text).to include(medication_name)
    expect(name_link['class']).to include('break-words')

    supply_badge = rendered.css('span').find { |span| span.text.squish == '12 units' }
    expect(supply_badge['class']).to include('whitespace-nowrap')
    expect(supply_badge['class']).to include('shrink-0')
  end

  def render_location
    vc = view_context
    policy_stub = Struct.new(:update?).new(false)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(location: location))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
end
