# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::CurrentUserContext do
  subject(:component) { component_class.new(current_user: current_user) }

  let(:current_user) { build_user(name: 'Admin Doe') }
  let(:component_class) do
    Class.new(Components::Base) do
      include Components::Layouts::CurrentUserContext
    end
  end

  describe '#authenticated?' do
    it 'is true when current_user is present' do
      expect(component.send(:authenticated?)).to be(true)
    end

    context 'when current_user is nil' do
      let(:current_user) { nil }

      it 'is false' do
        expect(component.send(:authenticated?)).to be(false)
      end
    end
  end

  describe '#user_is_admin?' do
    after { Current.reset }

    it 'is true for household owners' do
      Current.membership = build_membership(owner: true, administrator: false)

      expect(component.send(:user_is_admin?)).to be(true)
    end

    context 'when current membership is not a household manager' do
      before { Current.membership = build_membership(owner: false, administrator: false) }

      let(:current_user) { build_user(name: 'Carer Person') }

      it 'is false' do
        expect(component.send(:user_is_admin?)).to be(false)
      end
    end
  end

  describe '#current_membership_role_name' do
    after { Current.reset }

    it 'returns the active household membership role' do
      Current.membership = instance_double(HouseholdMembership, role: 'administrator')

      expect(component.send(:current_membership_role_name)).to eq('Administrator')
    end

    it 'falls back to member when tenant context is missing' do
      expect(component.send(:current_membership_role_name)).to eq('Member')
    end
  end

  describe '#current_user_name' do
    it 'returns the delegated user name' do
      expect(component.send(:current_user_name)).to eq('Admin Doe')
    end

    context 'when current_user is nil' do
      let(:current_user) { nil }

      it 'returns nil' do
        expect(component.send(:current_user_name)).to be_nil
      end
    end
  end

  def build_user(name:)
    Class.new do
      attr_reader :name

      def initialize(name:)
        @name = name
      end
    end.new(name: name)
  end

  def build_membership(owner:, administrator:)
    instance_double(HouseholdMembership, owner?: owner, administrator?: administrator)
  end
end
