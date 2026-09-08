require 'rails_helper'

RSpec.describe RemoveMedicationStockService do
  fixtures :households, :medications, :users

  subject(:remove_stock) { described_class.new.call(medication: medication, **attributes) }

  let(:medication) { medications(:paracetamol) }
  let(:submission_id) { SecureRandom.uuid }
  let(:attributes) { { quantity: '1', reason: 'dropped', note: 'Dropped during setup', submission_id: submission_id } }
  let(:events) { PaperTrail::Version.where(item_type: 'MedicationStockRemoval', item_id: medication.id) }

  it 'subtracts stock and records a distinct event without administration' do
    expect { remove_stock }.not_to change(MedicationTake, :count)

    expect(medication.reload.current_supply).to eq(79)
    expect(events.count).to eq(1)
    expect(JSON.parse(events.last.object)).to include(
      'quantity' => '1', 'reason' => 'dropped', 'note' => 'Dropped during setup',
      'previous_quantity' => '80', 'remaining_quantity' => '79', 'unit' => 'units'
    )
    expect(events.last.household_id).to eq(medication.household_id)
  end

  it 'accepts a fractional quantity with stored precision' do
    attributes[:quantity] = '0.25'
    expect(remove_stock).to be_success
    expect(medication.reload.current_supply).to eq(BigDecimal('79.75'))
  end

  ['', 'abc', 'NaN', 'Infinity', '-1', '0', '0.001', '81'].each do |quantity|
    it "rejects invalid or excessive quantity #{quantity.inspect}" do
      attributes[:quantity] = quantity
      expect(remove_stock).not_to be_success
      expect(medication.reload.current_supply).to eq(80)
      expect(events).to be_empty
    end
  end

  it 'rejects an unknown reason' do
    attributes[:reason] = 'taken'
    expect(remove_stock).not_to be_success
    expect(events).to be_empty
  end

  it 'rejects an overlong note' do
    attributes[:note] = 'x' * 1001
    expect(remove_stock).not_to be_success
  end

  it 'rejects an invalid submission identifier' do
    attributes[:submission_id] = ''
    expect(remove_stock).not_to be_success
  end

  it 'rejects untracked inventory' do
    medication.update!(current_supply: nil)
    expect(remove_stock).not_to be_success
    expect(medication.reload.current_supply).to be_nil
  end

  it 'replays a successful submission without removing stock again' do
    expect(remove_stock).to be_success
    expect(described_class.new.call(medication: medication, **attributes)).to be_success
    expect(medication.reload.current_supply).to eq(79)
    expect(events.count).to eq(1)
  end

  it 'rejects a changed payload using the same submission identifier' do
    remove_stock
    attributes[:quantity] = '2'
    expect(described_class.new.call(medication: medication, **attributes)).not_to be_success
    expect(medication.reload.current_supply).to eq(79)
  end

  it 'checks current stock after another request changed the stale record' do
    stale_medication = Medication.find(medication.id)
    attributes[:quantity] = '60'
    expect(remove_stock).to be_success
    result = described_class.new.call(medication: stale_medication,
                                      **attributes.merge(submission_id: SecureRandom.uuid))
    expect(result).not_to be_success
    expect(medication.reload.current_supply).to eq(20)
  end

  it 'rolls stock back if the audit event cannot be persisted' do
    allow(Audit::VersionEvent).to receive(:record!).and_raise(ActiveRecord::RecordInvalid)
    expect { remove_stock }.to raise_error(ActiveRecord::RecordInvalid)
    expect(medication.reload.current_supply).to eq(80)
    expect(events).to be_empty
  end

  context 'with dosage-specific stock' do
    let!(:dosage) do
      medication.dosage_records.create!(amount: 1, unit: 'tablet', frequency: 'daily', current_supply: 10,
                                        default_max_daily_doses: 4, default_min_hours_between_doses: 4,
                                        default_dose_cycle: :daily)
    end

    it 'requires a source instead of modifying the aggregate' do
      expect(remove_stock).not_to be_success
      expect(dosage.reload.current_supply).to eq(10)
    end

    it 'reduces the selected source and synchronises the aggregate' do
      attributes[:dosage_id] = dosage.id.to_s
      expect(remove_stock).to be_success
      expect(dosage.reload.current_supply).to eq(9)
      expect(medication.reload.current_supply).to eq(9)
      expect(JSON.parse(events.last.object)).to include('unit' => 'tablet', 'dosage_id' => dosage.id.to_s)
    end

    it 'rejects a dosage outside the medicine' do
      attributes[:dosage_id] = '-1'
      expect(remove_stock).not_to be_success
      expect(dosage.reload.current_supply).to eq(10)
    end
  end
end
