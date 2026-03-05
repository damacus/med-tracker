# frozen_string_literal: true

class ApplicationRecord < ActiveRecord::Base
  primary_abstract_class

  DOSE_CYCLE_OPTIONS = [
    %w[Daily daily],
    %w[Weekly weekly],
    %w[Monthly monthly]
  ].freeze
end
