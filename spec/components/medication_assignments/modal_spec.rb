# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::MedicationAssignments::Modal, type: :component do
  it 'renders the back action as a shared text link' do
    back_link = rendered_back_link
    classes = back_link['class'].split

    expect(back_link.text).to include('Back')
    expect(back_link['data-turbo-frame']).to eq('modal')
    expect(classes).to include('state-layer')
    expect(classes).not_to include('rounded-xl')
  end

  def rendered_back_link
    person = create(:person, name: 'Alex Patient')
    medication = create(:medication, name: 'Paracetamol')
    assignment = MedicationAssignment.new(medication_id: medication.id)
    back_path = route_helpers.add_medication_person_path(person, household_slug: 'test-household')
    rendered = render_inline(
      described_class.new(assignment: assignment, person: person, medications: [medication],
                          back_path: back_path)
    )

    rendered.at_css("a[href='#{back_path}']")
  end

  def route_helpers = Rails.application.routes.url_helpers
end
