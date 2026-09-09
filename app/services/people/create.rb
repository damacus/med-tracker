module People
  class Create
    def initialize(person:, authorization:, request: nil)
      @person = person
      @authorization = authorization
      @request = request
    end

    def call
      ActiveRecord::Base.transaction do
        if @authorization.membership.person.present? && (@person.minor? || @person.dependent_adult?)
          CareDelegation::Assign.new(carer: @authorization.membership.person, patient: @person,
                                     relationship_type: :family_member,
                                     granted_by_membership: @authorization.membership).call
        else
          @person.save!
          grant_access!
        end
      end
      @person
    end

    private

    def grant_access!
      Households::AccessChange.new(actor_account: @authorization.account,
                                   actor_membership: @authorization.membership, request: @request).create_grant!(
                                     household: @authorization.household,
                                     household_membership: @authorization.membership, person: @person,
                                     access_level: :manage, relationship_type: :family_member,
                                     granted_by_membership: @authorization.membership
                                   )
    end
  end
end
