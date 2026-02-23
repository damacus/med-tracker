# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::People::ShowView, type: :component do
  fixtures :accounts, :people, :users, :locations, :medicines, :dosages, :prescriptions,
           :person_medicines, :medication_takes

  let(:person) { people(:jane) }
  let(:prescriptions) { person.prescriptions }
  let(:person_medicines) { person.person_medicines }

  def render_view
    view_context_helper = view_context
    stub_view_context_helpers(view_context_helper)

    html = view_context_helper.render(described_class.new(
                                        person: person,
                                        prescriptions: prescriptions,
                                        person_medicines: person_medicines
                                      ))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  private

  def stub_view_context_helpers(view_context_helper)
    admin = users(:admin)
    policy_stub = Struct.new(:update?, :create?, :show?, :take_medicine?, :destroy?).new(true, true, true, true, true)
    view_context_helper.singleton_class.define_method(:policy) { |_record| policy_stub }
    view_context_helper.singleton_class.define_method(:current_user) { admin }
    view_context_helper.singleton_class.define_method(:pundit_user) { admin }
  end

  it 'renders the person name and type' do
    rendered = render_view
    expect(rendered.text).to include(person.name)
    expect(rendered.text).to include('Adult')
  end

  it 'renders the Profile Overview card' do
    rendered = render_view
    expect(rendered.text).to include('Profile Overview')
    expect(rendered.text).to include('Date of Birth')
  end

  it 'renders the Care Actions card' do
    rendered = render_view
    expect(rendered.text).to include('Care Actions')
    expect(rendered.text).to include('Add Prescription')
  end

  it 'renders prescriptions and my medicines sections' do
    rendered = render_view
    expect(rendered.text).to include('Prescriptions')
    expect(rendered.text).to include('My Medicines')
  end
end
