# frozen_string_literal: true

require 'json'

namespace :storage do
  desc 'Run a bounded portable-storage migration operation'
  task migration: :environment do
    status = StorageMigration::Command.new.call
    exit(status) unless status.zero?
  end
end
