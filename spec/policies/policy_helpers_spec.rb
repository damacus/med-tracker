# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PolicyHelpers, type: :policy do
  fixtures :all

  let(:host_class) do
    Class.new(ApplicationPolicy) do
      def admin_public? = admin?
      def admin_or_clinician_public? = admin_or_clinician?
      def medical_staff_public? = medical_staff?
      def carer_or_parent_public? = carer_or_parent?
      def doctor_public? = doctor?
      def nurse_public? = nurse?
    end
  end

  def policy_for(user) = host_class.new(user, :record)

  it 'admin? is true only for administrators' do
    expect(policy_for(users(:admin)).admin_public?).to be(true)
    expect(policy_for(users(:doctor)).admin_public?).to be(false)
  end

  it 'admin_or_clinician? covers admin, doctor and nurse' do
    aggregate_failures do
      expect(policy_for(users(:admin)).admin_or_clinician_public?).to be(true)
      expect(policy_for(users(:doctor)).admin_or_clinician_public?).to be(true)
      expect(policy_for(users(:nurse)).admin_or_clinician_public?).to be(true)
      expect(policy_for(users(:carer)).admin_or_clinician_public?).to be(false)
    end
  end

  it 'medical_staff? covers doctor and nurse only' do
    aggregate_failures do
      expect(policy_for(users(:doctor)).medical_staff_public?).to be(true)
      expect(policy_for(users(:nurse)).medical_staff_public?).to be(true)
      expect(policy_for(users(:admin)).medical_staff_public?).to be(false)
    end
  end

  it 'carer_or_parent? covers carer and parent only' do
    aggregate_failures do
      expect(policy_for(users(:carer)).carer_or_parent_public?).to be(true)
      expect(policy_for(users(:parent)).carer_or_parent_public?).to be(true)
      expect(policy_for(users(:admin)).carer_or_parent_public?).to be(false)
    end
  end

  it 'doctor? is true only for doctors' do
    expect(policy_for(users(:doctor)).doctor_public?).to be(true)
    expect(policy_for(users(:nurse)).doctor_public?).to be(false)
    expect(policy_for(users(:admin)).doctor_public?).to be(false)
  end

  it 'nurse? is true only for nurses' do
    expect(policy_for(users(:nurse)).nurse_public?).to be(true)
    expect(policy_for(users(:doctor)).nurse_public?).to be(false)
    expect(policy_for(users(:admin)).nurse_public?).to be(false)
  end

  it 'returns false (not nil) when there is no user' do
    aggregate_failures do
      expect(policy_for(nil).admin_public?).to be(false)
      expect(policy_for(nil).admin_or_clinician_public?).to be(false)
      expect(policy_for(nil).medical_staff_public?).to be(false)
      expect(policy_for(nil).carer_or_parent_public?).to be(false)
      expect(policy_for(nil).doctor_public?).to be(false)
      expect(policy_for(nil).nurse_public?).to be(false)
    end
  end
end
