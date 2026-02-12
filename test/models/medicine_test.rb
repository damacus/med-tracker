# frozen_string_literal: true

require 'test_helper'

class MedicineTest < ActiveSupport::TestCase
  test 'valid medicine' do
    medicine = medicines(:paracetamol)
    assert medicine.valid?
  end

  test 'requires name' do
    medicine = Medicine.new(name: nil, reorder_threshold: 50)
    assert_not medicine.valid?
    assert_includes medicine.errors[:name], "can't be blank"
  end

  test 'does not require current_supply' do
    medicine = Medicine.new(name: 'Test', current_supply: nil, reorder_threshold: 50)
    assert medicine.valid?
  end

  test 'current_supply must be integer >= 0 when present' do
    medicine = Medicine.new(name: 'Test', reorder_threshold: 50)

    medicine.current_supply = -1
    assert_not medicine.valid?

    medicine.current_supply = 1.5
    assert_not medicine.valid?

    medicine.current_supply = 0
    assert medicine.valid?
  end

  test 'stock must be integer >= 0 when present' do
    medicine = Medicine.new(name: 'Test', reorder_threshold: 50)

    medicine.stock = -1
    assert_not medicine.valid?

    medicine.stock = nil
    assert medicine.valid?

    medicine.stock = 0
    assert medicine.valid?
  end

  test 'reorder_threshold must be integer >= 0' do
    medicine = Medicine.new(name: 'Test', reorder_threshold: -1)
    assert_not medicine.valid?

    medicine.reorder_threshold = 0
    assert medicine.valid?
  end

  test 'has_many dosages' do
    medicine = medicines(:paracetamol)
    assert_respond_to medicine, :dosages
    assert_kind_of ActiveRecord::Associations::CollectionProxy, medicine.dosages
  end

  test 'has_many prescriptions' do
    medicine = medicines(:paracetamol)
    assert_respond_to medicine, :prescriptions
    assert_kind_of ActiveRecord::Associations::CollectionProxy, medicine.prescriptions
  end

  test 'destroying medicine destroys dependent dosages' do
    medicine = medicines(:paracetamol)
    dosage_count = medicine.dosages.count

    assert_difference('Dosage.count', -dosage_count) do
      medicine.destroy
    end
  end

  test 'low_stock? returns true when stock is below reorder threshold' do
    medicine = Medicine.new(name: 'Test', stock: 25, reorder_threshold: 50)
    assert medicine.low_stock?
  end

  test 'low_stock? returns true when stock equals reorder threshold' do
    medicine = Medicine.new(name: 'Test', stock: 50, reorder_threshold: 50)
    assert medicine.low_stock?
  end

  test 'low_stock? returns false when stock is above reorder threshold' do
    medicine = Medicine.new(name: 'Test', stock: 75, reorder_threshold: 50)
    assert_not medicine.low_stock?
  end

  test 'low_stock? returns false when stock is nil' do
    medicine = Medicine.new(name: 'Test', stock: nil, reorder_threshold: 50)
    assert_not medicine.low_stock?
  end

  test 'out_of_stock? returns true when stock is 0' do
    medicine = Medicine.new(stock: 0)
    assert medicine.out_of_stock?
  end

  test 'out_of_stock? returns false when stock is positive' do
    medicine = Medicine.new(stock: 1)
    assert_not medicine.out_of_stock?
  end

  test 'out_of_stock? returns false when stock is nil' do
    medicine = Medicine.new(stock: nil)
    assert_not medicine.out_of_stock?
  end
end
