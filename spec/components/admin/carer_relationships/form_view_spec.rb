# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::CarerRelationships::FormView, type: :component do
  fixtures :people, :carer_relationships

  let(:relationship) { CarerRelationship.new }
  let(:carers) { [people(:jane)] }
  let(:patients) { [people(:child_patient)] }

  it 'uses explicit surface text tokens for dark mode legibility' do
    rendered = render_inline(described_class.new(relationship:, carers:, patients:))

    expect(rendered.css('h1.text-foreground').text).to include('New Carer Relationship')
    expect(rendered.css('p.text-on-surface-variant').text).to include('Assign a carer to a patient.')

    %w[
      carer_relationship_carer_id
      carer_relationship_patient_id
      carer_relationship_relationship_type
    ].each do |field_id|
      expect(rendered.css("label[for='#{field_id}'].text-on-surface-variant")).to be_present
    end
  end
end
