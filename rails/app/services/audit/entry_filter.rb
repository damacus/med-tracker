# frozen_string_literal: true

module Audit
  class EntryFilter
    class Invalid < StandardError; end

    def initialize(environment, relation: AuditLedgerEntry.all)
      @environment = environment
      @relation = relation
    end

    def call
      filtered = relation
      filtered = filtered.where(household_id: integer_filter('HOUSEHOLD_ID')) if value?('HOUSEHOLD_ID')
      filtered = filtered.where(occurred_at: time_filter('FROM')..) if value?('FROM')
      filtered = filtered.where(occurred_at: ..time_filter('TO')) if value?('TO')
      filtered
    end

    def time_filtered?
      value?('FROM') || value?('TO')
    end

    def household_id
      integer_filter('HOUSEHOLD_ID') if value?('HOUSEHOLD_ID')
    end

    private

    attr_reader :environment, :relation

    def value?(name)
      environment[name].present?
    end

    def integer_filter(name)
      Integer(environment.fetch(name), 10)
    rescue ArgumentError
      raise Invalid, "invalid #{name}"
    end

    def time_filter(name)
      Time.iso8601(environment.fetch(name))
    rescue ArgumentError
      raise Invalid, "invalid #{name}"
    end
  end
end
