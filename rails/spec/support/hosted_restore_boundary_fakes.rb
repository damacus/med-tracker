# frozen_string_literal: true

module HostedRestoreBoundaryFakes
  class Connection
    attr_reader :household_id, :transaction_calls

    def initialize
      @transaction_calls = 0
      @household_id = nil
      @in_transaction = false
    end

    def transaction(requires_new:)
      previous = [@in_transaction, @household_id]
      @transaction_calls += 1
      @in_transaction = requires_new
      yield
    ensure
      @in_transaction, @household_id = previous
    end

    def execute(sql)
      raise 'set_config called outside a transaction' unless @in_transaction

      value = Array(sql.scan(/'([^']*)'/).last).first.to_s
      @household_id = value.presence&.to_i
    end

    def select_value(sql)
      sql.include?('current_user') ? 'med_tracker_app' : '20260714090000'
    end

    def select_one(_sql)
      { 'relrowsecurity' => true, 'relforcerowsecurity' => true }
    end

    def quote(value)
      "'#{value}'"
    end
  end

  class Model
    def initialize(connection, ids_by_household)
      @connection = connection
      @ids_by_household = ids_by_household
    end

    def where(household_id:)
      household_ids = Array(household_id).map(&:to_i)
      Relation.new(household_ids.include?(connection.household_id) ? visible_ids : [])
    end

    def exists?(id)
      visible_ids.include?(id)
    end

    private

    attr_reader :connection, :ids_by_household

    def visible_ids
      [ids_by_household[connection.household_id]].compact
    end
  end

  class Relation
    def initialize(ids)
      @ids = ids
    end

    def pick(_column)
      ids.first
    end

    def none?
      ids.empty?
    end

    private

    attr_reader :ids
  end
end
