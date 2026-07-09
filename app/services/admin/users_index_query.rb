# frozen_string_literal: true

module Admin
  class UsersIndexQuery
    attr_reader :scope, :filters, :household

    def initialize(scope:, filters:, household: nil)
      @scope = scope
      @filters = filters
      @household = household
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
      when 'membership_role'
        order_by_membership_role(relation)
      else
        relation.order(created_at: direction_symbol)
      end
    end

    def filtered_scope
      relation = scope.includes(person: { account: :platform_admin })
      relation = apply_search(relation) if search.present?
      relation = apply_membership_role_filter(relation) if membership_role.present?
      relation = apply_status_filter(relation) if status.present?
      relation
    end

    def search
      filters[:search].to_s.presence
    end

    def membership_role
      filters[:membership_role].to_s.presence
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

    def apply_membership_role_filter(relation)
      return relation.none if household.blank?

      household_membership_join(relation).where(
        household_memberships: {
          household_id: household.id,
          role: membership_role
        }
      )
    end

    def household_membership_join(relation)
      relation.left_joins(person: { account: :household_memberships })
    end

    def order_by_membership_role(relation)
      household_membership_join(relation).order(HouseholdMembership.arel_table[:role].public_send(direction_symbol))
    end

    def sort_column
      sort.presence_in(%w[name email created_at membership_role]) || 'created_at'
    end

    def direction_symbol
      { 'desc' => :desc }.fetch(direction, :asc)
    end
  end

  class UserAccessSummaryQuery
    Result = Data.define(:membership_roles_by_account_id, :platform_admin_account_ids) do
      def membership_role_for(account_id)
        membership_roles_by_account_id[account_id]
      end

      def platform_admin?(account_id)
        platform_admin_account_ids.include?(account_id)
      end
    end

    attr_reader :users, :household

    def initialize(users:, household: nil)
      @users = users
      @household = household
    end

    def call
      Result.new(
        membership_roles_by_account_id: membership_roles_by_account_id,
        platform_admin_account_ids: platform_admin_account_ids
      )
    end

    private

    def membership_roles_by_account_id
      membership_scope.index_by(&:account_id).transform_values(&:role)
    end

    def membership_scope
      scope = household ? household.household_memberships : HouseholdMembership.all
      scope.active.where(account_id: account_ids).order(:id)
    end

    def platform_admin_account_ids
      PlatformAdmin.active.where(account_id: account_ids).pluck(:account_id).to_set
    end

    def account_ids
      @account_ids ||= users.filter_map { |user| user.person&.account_id }.uniq
    end
  end
end
