# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::MedicationWorkflow::PersonSelection, type: :component do
  it 'renders person choices with the shared M3 link contract' do
    person = create(:person, name: 'Alex Patient')

    rendered = render_inline(described_class.new(people: [person]))
    choice = rendered.at_css("a[href='#{person_assignment_path(person)}']")
    classes = choice['class'].split

    expect(choice['data-turbo-frame']).to eq('modal')
    expect(classes).to include('state-layer', 'rounded-2xl')
    expect(classes).to include_touch_target_class
    expect(choice.text).to include('Alex Patient')
  end

  def include_touch_target_class
    satisfy { |classes| classes.include?('min-h-11') || classes.include?('min-h-[44px]') }
  end

  def person_assignment_path(person)
    Rails.application.routes.url_helpers.new_person_medication_assignment_path(
      person,
      source: :workflow,
      household_slug: 'test-household'
    )
  end
end
