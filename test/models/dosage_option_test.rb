require "test_helper"

class DosageOptionTest < ActiveSupport::TestCase
  def setup
    @medicine = medicines(:paracetamol)
    @dosage_option = dosage_options(:paracetamol_500)
  end

  test "valid dosage option" do
    assert @dosage_option.valid?
  end

  test "requires amount" do
    @dosage_option.amount = nil
    assert_not @dosage_option.valid?
    assert_includes @dosage_option.errors[:amount], "can't be blank"
  end

  test "requires positive amount" do
    @dosage_option.amount = 0
    assert_not @dosage_option.valid?
    assert_includes @dosage_option.errors[:amount], "must be greater than 0"

    @dosage_option.amount = -1
    assert_not @dosage_option.valid?
    assert_includes @dosage_option.errors[:amount], "must be greater than 0"
  end

  test "requires unique amount per medicine" do
    duplicate = @medicine.dosage_options.build(amount: @dosage_option.amount)
    assert_not duplicate.valid?
    assert_includes duplicate.errors[:amount], "has already been taken"
  end

  test "allows same amount for different medicines" do
    other_medicine = medicines(:ibuprofen)
    duplicate = other_medicine.dosage_options.build(amount: @dosage_option.amount)
    assert duplicate.valid?
  end

  test "ordered scope returns dosage options in ascending order" do
    ordered_amounts = DosageOption.ordered.pluck(:amount)
    assert_equal ordered_amounts, ordered_amounts.sort
  end
end
