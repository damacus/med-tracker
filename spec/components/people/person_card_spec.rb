# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::People::PersonCard, type: :component do
  let(:person) do
    build(:person, :dependent_adult, id: 1, name: 'Jane Doe')
  end

  it 'renders Needs Carer badge using RubyUI::Badge component (not inline styles)' do
    rendered = render_inline(described_class.new(person: person))

    badge = rendered.at_css('[data-testid="needs-carer-badge"]')
    expect(badge).to be_present, 'Expected Needs Carer badge to be rendered'
    expect(badge.name).to eq('span')
    expect(badge['class']).to include('ring-warning/20'),
                              'Expected badge to use RubyUI::Badge warning variant classes'
  end
end
