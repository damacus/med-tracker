# frozen_string_literal: true

module RetirableAdministrationSource
  extend ActiveSupport::Concern

  included do
    has_many :medication_takes, dependent: :restrict_with_error
    scope :current, -> { where(retired_at: nil) }
  end

  def retire! = update!(retired_at: Time.current, active: false)
end
