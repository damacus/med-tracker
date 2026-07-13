# frozen_string_literal: true

module Households
  class AccessChange
    Result = Data.define(:success?, :record, :outcome)
    Rejected = Class.new(StandardError) do
      attr_reader :record

      def initialize(record)
        @record = record
        super(record.errors.full_messages.to_sentence)
      end
    end

    def self.for(membership, request: nil)
      new(actor_account: membership.account, actor_membership: membership, request: request)
    end

    def initialize(actor_account:, actor_membership:, request:)
      @context = {
        actor_account: actor_account,
        actor_membership: actor_membership,
        request: request
      }
    end

    def update_membership(membership, attributes)
      Membership.new(**context, membership: membership, attributes: attributes).call
    end

    def promote_owner(membership)
      update_membership(membership, role: :owner)
    end

    def update_membership!(membership, attributes)
      successful_record!(update_membership(membership, attributes))
    end

    def create_grant(attributes)
      change_grant(PersonAccessGrant.new, attributes)
    end

    def create_grant!(attributes)
      successful_record!(create_grant(attributes))
    end

    def update_grant(grant, attributes)
      change_grant(grant, attributes)
    end

    def update_grant!(grant, attributes)
      successful_record!(update_grant(grant, attributes))
    end

    def upsert_grant!(grant, attributes)
      return create_grant!(attributes) if grant.new_record?

      update_grant!(grant, attributes)
    end

    def revoke_grant(grant)
      update_grant(grant, revoked_at: grant.revoked_at || Time.current)
    end

    def revoke_grant!(grant)
      successful_record!(revoke_grant(grant))
    end

    private

    attr_reader :context

    def successful_record!(result)
      return result.record if result.success?

      raise ActiveRecord::RecordInvalid, result.record
    end

    def change_grant(grant, attributes)
      PersonGrant.new(**context, grant: grant, attributes: attributes).call
    end
  end
end
