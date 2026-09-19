# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::AmbiguousPersonAccessGrantClassifier do
  subject(:classify) do
    described_class.new(grant: grant, actor_membership: actor_membership)
                   .call(decision: decision, reason: reason,
                         carer_relationship_id: relationship&.id)
  end

  let(:household) { create(:household) }
  let(:actor_membership) { create_membership(create_account_person(household, 'actor'), role: :owner) }
  let(:relationship) do
    CarerRelationship.create!(household: household, carer: grant.household_membership.person,
                              patient: grant.person, relationship_type: :parent, active: true)
  end
  let(:grant) do
    carer = create_account_person(household, 'carer')
    household.person_access_grants.create!(
      household_membership: create_membership(carer),
      person: create(:person, household: household),
      access_level: :manage,
      relationship_type: :parent,
      granted_by_membership: actor_membership
    )
  end

  describe 'manual decision' do
    let(:decision) { 'manual' }

    it 'marks the grant as confirmed independent authority' do
      classify

      expect(grant.reload).to have_attributes(
        disposition: 'manual',
        carer_relationship_id: nil,
        revoked_at: nil,
        classified_by_membership_id: actor_membership.id,
        classification_reason: reason
      )
      expect(grant.classified_at).to be_present
    end

    it 'leaves the grant out of the ambiguous queue' do
      relationship
      queue = Admin::AmbiguousPersonAccessGrantsIndexQuery.new(
        scope: PersonAccessGrant.where(household: household)
      )

      expect(queue.call.map(&:id)).to include(grant.id)
      classify
      expect(queue.call.map(&:id)).not_to include(grant.id)
    end
  end

  describe 'attach decision' do
    let(:decision) { 'attach' }

    it 'attributes the grant to the compatible relationship' do
      classify

      expect(grant.reload).to have_attributes(
        carer_relationship_id: relationship.id,
        revoked_at: nil,
        classified_by_membership_id: actor_membership.id,
        classification_reason: reason
      )
    end

    it 'rejects a relationship with an incompatible type' do
      relationship.update!(relationship_type: 'professional_carer')

      expect { classify }.to raise_error(described_class::StaleDecision)
      expect(grant.reload.carer_relationship_id).to be_nil
    end

    it 'rejects an inactive relationship' do
      relationship.update!(active: false)

      expect { classify }.to raise_error(described_class::StaleDecision)
    end

    it 'rejects a relationship for a different pair' do
      relationship.update!(patient: create(:person, household: household))

      expect { classify }.to raise_error(described_class::StaleDecision)
    end

    it 'rejects a relationship from another household' do
      other_household = create(:household)
      foreign_relationship = CarerRelationship.create!(
        household: other_household,
        carer: create(:person, household: other_household),
        patient: create(:person, household: other_household),
        relationship_type: :parent,
        active: true
      )

      expect { classify_with(decision: 'attach', carer_relationship_id: foreign_relationship.id) }
        .to raise_error(described_class::StaleDecision)
    end

    it 'rejects when no relationship is supplied' do
      expect { classify_with(decision: 'attach', carer_relationship_id: nil) }
        .to raise_error(described_class::StaleDecision)
    end
  end

  describe 'revoke decision' do
    let(:decision) { 'revoke' }

    it 'revokes the grant and records the decision' do
      classify

      expect(grant.reload).to have_attributes(
        carer_relationship_id: nil,
        classified_by_membership_id: actor_membership.id,
        classification_reason: reason
      )
      expect(grant.revoked_at).to be_present
    end
  end

  describe 'decision validation' do
    it 'rejects a blank reason for an authority-changing decision' do
      %w[attach manual revoke].each do |authority_decision|
        expect { classify_with(decision: authority_decision, decision_reason: ' ') }
          .to raise_error(described_class::InvalidDecision)
      end
    end

    it 'rejects an unknown decision' do
      expect { classify_with(decision: 'merge') }
        .to raise_error(described_class::InvalidDecision)
    end
  end

  describe 'stale grant state' do
    let(:decision) { 'manual' }

    it 'rejects an already revoked grant' do
      grant.update!(revoked_at: Time.current)

      expect { classify }.to raise_error(described_class::StaleDecision)
    end

    it 'rejects an expired grant' do
      grant.update!(expires_at: 1.hour.ago)

      expect { classify }.to raise_error(described_class::StaleDecision)
    end

    it 'rejects a grant already owned by a relationship' do
      grant.update!(carer_relationship: relationship)

      expect { classify }.to raise_error(described_class::StaleDecision)
    end

    it 'rejects an already classified grant' do
      grant.update!(disposition: 'manual', classified_at: Time.current,
                    classified_by_membership: actor_membership, classification_reason: 'earlier')

      expect { classify }.to raise_error(described_class::StaleDecision)
    end
  end

  def reason
    'Reviewed: grant was issued before delegation workflows existed'
  end

  def classify_with(decision: 'manual', decision_reason: reason, carer_relationship_id: relationship&.id)
    described_class.new(grant: grant, actor_membership: actor_membership)
                   .call(decision: decision, reason: decision_reason,
                         carer_relationship_id: carer_relationship_id)
  end

  def create_account_person(target_household, prefix)
    account = Account.create!(
      email: "#{prefix}-#{SecureRandom.hex(4)}@example.com",
      password_hash: BCrypt::Password.create('password'),
      status: :verified
    )
    create(:person, household: target_household, account: account)
  end

  def create_membership(person, role: :member)
    person.household.household_memberships.create!(
      account: person.account,
      person: person,
      role: role,
      status: :active
    )
  end
end
