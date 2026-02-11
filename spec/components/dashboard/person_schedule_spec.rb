# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PersonSchedule, type: :component do
  fixtures :accounts, :people, :users, :medicines, :dosages, :prescriptions

  subject(:component) { described_class.new(person: person, prescriptions: prescriptions) }

  let(:person) { people(:john) }
  let(:prescriptions) { person.prescriptions.where(active: true) }

  before do
    MedicationTake.where(prescription: prescriptions).delete_all
  end

  it 'renders the person\'s name' do
    rendered = render_inline(component)
    expect(rendered.text).to include(person.name)
  end

  it 'renders each prescription' do
    rendered = render_inline(component)

    prescriptions.each do |prescription|
      prescription_element = rendered.css("#prescription_#{prescription.id}")
      expect(prescription_element).to be_present
      expect(rendered.text).to include(prescription.medicine.name)
    end
  end

  it 'renders take now links for each prescription' do
    rendered = render_inline(component)

    prescriptions.each do |prescription|
      link = rendered.css("[data-test-id='take-medicine-#{prescription.id}']")
      expect(link).to be_present
      expect(link.text).to include('Take Now')
    end
  end
end
