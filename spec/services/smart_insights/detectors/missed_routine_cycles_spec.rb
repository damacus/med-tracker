require 'rails_helper'

RSpec.describe SmartInsights::Detectors::MissedRoutineCycles do
  let(:source) { build_stubbed(:person_medication, :routine, dose_cycle: :weekly) }

  before { travel_to(Time.zone.local(2026, 9, 9, 12)) }

  def cycle(offset, missed: 1, source: self.source)
    first = Date.new(2026, 8, 17) + offset.weeks
    { source: source, window_starts_on: first, window_ends_on: first + 6,
      expected: 1, actual: 0, not_taken: 1 - missed, unexplained_missed: missed }
  end

  def insights(cycles)
    context = instance_double(SmartInsights::Context, cycle_summaries: cycles)
    described_class.new(context).call
  end

  it 'warns about consecutive completed cycles with unexplained misses' do
    expect(insights([cycle(0), cycle(1)]).sole).to have_attributes(
      key: :missed_routine_cycles, family: :adherence, severity: :warning, metric_value: '2 cycles'
    )
  end

  it 'breaks the pattern at an explained not-taken cycle' do
    expect(insights([cycle(0), cycle(1, missed: 0), cycle(2)])).to be_empty
  end

  it 'does not join separated or different-source cycles' do
    other = build_stubbed(:person_medication, :routine)
    expect(insights([cycle(0), cycle(2)])).to be_empty
    expect(insights([cycle(0), cycle(1, source: other)])).to be_empty
  end

  it 'ignores an unfinished cycle even if its input claims a miss' do
    expect(insights([cycle(2), cycle(3)])).to be_empty
  end
end
