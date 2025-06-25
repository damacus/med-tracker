# frozen_string_literal: true

require "rails_helper"

RSpec.describe Components::Dashboard::PersonSchedule, type: :component do
  fixtures :users, :medicines, :dosages, :prescriptions

  let(:user) { users(:john) }
  let(:prescriptions) { user.prescriptions.where(active: true) }
  subject { described_class.new(user: user, prescriptions: prescriptions) }

  it "renders the person's name and age" do
    rendered = render_inline(subject)
    
    expect(rendered).to have_css(".schedule-person__name", text: user.name)
    expect(rendered).to have_css(".schedule-person__age", text: "Age: #{user.age}")
  end
  
  it "renders each prescription" do
    rendered = render_inline(subject)
    
    prescriptions.each do |prescription|
      expect(rendered).to have_css("#prescription_#{prescription.id}")
      expect(rendered).to have_css(".prescription-card__medicine", text: prescription.medicine.name)
    end
  end

  it "renders take now buttons for each prescription" do
    rendered = render_inline(subject)
    
    prescriptions.each do |prescription|
      expect(rendered).to have_css("[data-test-id='take-medicine-#{prescription.id}']", text: "Take Now")
    end
  end
end
