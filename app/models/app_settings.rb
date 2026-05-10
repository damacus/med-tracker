# frozen_string_literal: true

class AppSettings < ApplicationRecord
  has_paper_trail

  # Always a single row. Access via AppSettings.instance, never instantiate directly.
  def self.instance
    first_or_create!(invite_only: false)
  end

  # INVITE_ONLY env var takes precedence when set, letting ops pin the value
  # without touching the database.  When absent, the admin-configured DB value
  # is used.
  def self.invite_only?
    if ENV.key?('INVITE_ONLY')
      ActiveModel::Type::Boolean.new.cast(ENV.fetch('INVITE_ONLY'))
    else
      instance.invite_only
    end
  end

  # Returns :env when the value is locked by the environment variable,
  # :database otherwise.  Used by the admin UI to disable the toggle.
  def self.invite_only_source
    ENV.key?('INVITE_ONLY') ? :env : :database
  end
end
