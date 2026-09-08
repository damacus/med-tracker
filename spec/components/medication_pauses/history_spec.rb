require 'rails_helper'

RSpec.describe Components::MedicationPauses::History, type: :component do
  it 'shows newest periods first and explicitly labels missing legacy context' do
    older = MedicationPausePeriod.new(reason: 'reason_not_recorded', legacy_context: true, created_at: 2.days.ago)
    newer = MedicationPausePeriod.new(reason: 'other', note: '<private note>', started_at: 1.day.ago,
                                      ended_at: Time.current, created_at: 1.day.ago)

    rendered = render_inline(described_class.new(periods: [older, newer]))

    expect(rendered.text).to include('Start unknown', 'Reason not recorded', 'Person not recorded', '<private note>')
    expect(rendered.at_css('ol').text.index('Other')).to be < rendered.at_css('ol').text.index('Reason not recorded')
    expect(rendered.to_html).to include('&lt;private note&gt;')
  end
end
