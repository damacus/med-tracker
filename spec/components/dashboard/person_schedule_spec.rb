# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::PersonSchedule, type: :component do
  fixtures :people, :users, :medicines, :dosages, :prescriptions

  subject { described_class.new(person: person, prescriptions: prescriptions) }

  let(:person) { people(:john) }
  let(:prescriptions) { person.prescriptions.where(active: true) }

  it 'renders the person\'s name and age' do
    rendered = render_inline(subject)
    # Use Nokogiri methods instead of Capybara matchers
    name_element = rendered.css('.schedule-person__name')
    expect(name_element).to be_present
    expect(name_element.text).to include(person.name)

    age_element = rendered.css('.schedule-person__age')
    expect(age_element).to be_present
    expect(age_element.text).to include("Age: #{person.age}")
  end

  it 'renders each prescription' do
    rendered = render_inline(subject)

    prescriptions.each do |prescription|
      prescription_element = rendered.css("#prescription_#{prescription.id}")
      expect(prescription_element).to be_present

      medicine_element = prescription_element.css('.prescription-card__medicine')
      expect(medicine_element).to be_present
      expect(medicine_element.text).to include(prescription.medicine.name)
    end
  end

  it 'renders take now buttons for each prescription' do
    rendered = render_inline(subject)

    prescriptions.each do |prescription|
      button = rendered.css("[data-test-id='take-medicine-#{prescription.id}']")
      expect(button).to be_present
      expect(button.text).to include('Take Now')
    end
  end
end
