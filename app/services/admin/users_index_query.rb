# frozen_string_literal: true

module Admin
  class UsersIndexQuery
    attr_reader :scope, :filters

    def initialize(scope:, filters:)
      @scope = scope
      @filters = filters
    end

    def call
      apply_sorting(filtered_scope)
    end

    private

    def apply_sorting(relation)
      case sort_column
      when 'name'
        relation.left_joins(:person).order(Person.arel_table[:name].public_send(direction_symbol))
      when 'email'
        relation.order(email_address: direction_symbol)
      when 'role'
        relation.order(role: direction_symbol)
      else
        relation.order(created_at: direction_symbol)
      end
    end

    def filtered_scope
      relation = scope.includes(:person)
      relation = apply_search(relation) if search.present?
      relation = relation.where(role: role) if role.present?
      relation = apply_status_filter(relation) if status.present?
      relation
    end

    def search
      filters[:search].to_s.presence
    end

    def role
      filters[:role].to_s.presence
    end

    def status
      filters[:status].to_s.presence
    end

    def sort
      filters[:sort].to_s.presence
    end

    def direction
      filters[:direction].to_s.presence
    end

    def apply_search(relation)
      search_term = "%#{ActiveRecord::Base.sanitize_sql_like(search)}%"
      relation.joins(:person).where('people.name ILIKE ? OR users.email_address ILIKE ?', search_term, search_term)
    end

    def apply_status_filter(relation)
      case status
      when 'active'
        relation.active
      when 'inactive'
        relation.inactive
      when 'soft_deleted'
        soft_deleted_scope(relation)
      else
        relation
      end
    end

    def soft_deleted_scope(relation)
      account_join = relation.left_joins(person: :account)
      account_join.where(accounts: { id: nil }).or(account_join.where(accounts: { status: Account.statuses[:closed] }))
    end

    def sort_column
      sort.presence_in(%w[name email created_at role]) || 'created_at'
    end

    def direction_symbol
      { 'desc' => :desc }.fetch(direction, :asc)
    end
  end
end
