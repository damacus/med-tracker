# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Form, type: :component do
  it 'keeps formerly oversized form wrappers below their extraction thresholds' do
    oversized_forms.each do |relative_path, maximum_lines|
      expect(line_count(relative_path)).to be <= maximum_lines
    end
  end

  it 'keeps extracted field groups in standalone component files' do
    field_components.each do |relative_path|
      expect(Rails.root.join(relative_path)).to exist
    end
  end

  def line_count(relative_path)
    Rails.root.join(relative_path).read.lines.count
  end

  def oversized_forms
    {
      'app/components/schedules/form.rb' => 415,
      'app/components/medications/form_view.rb' => 400,
      'app/components/person_medications/form_fields.rb' => 330
    }
  end

  def field_components
    %w[
      app/components/schedules/fields.rb
      app/components/medications/identity_fields.rb
      app/components/medications/supply_fields.rb
      app/components/medications/dosage_options_fields.rb
      app/components/medications/warnings_field.rb
      app/components/person_medications/timing_fields.rb
      app/components/shared/field_hint.rb
    ]
  end
end
