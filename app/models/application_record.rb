# frozen_string_literal: true

class ApplicationRecord < ActiveRecord::Base
  primary_abstract_class

  DOSE_CYCLE_OPTIONS = %w[daily weekly monthly].map { |v| [v.humanize, v] }.freeze
end
