# frozen_string_literal: true

module PolicyHelpers
  extend ActiveSupport::Concern

  private

  def admin?
    user&.administrator? || false
  end

  def admin_or_clinician?
    user&.administrator? || user&.doctor? || user&.nurse? || false
  end

  def doctor?
    user&.doctor? || false
  end

  def nurse?
    user&.nurse? || false
  end

  def medical_staff?
    user&.doctor? || user&.nurse? || false
  end

  def carer_or_parent?
    user&.carer? || user&.parent? || false
  end
end
