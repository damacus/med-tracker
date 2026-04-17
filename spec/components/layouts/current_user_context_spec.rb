# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::CurrentUserContext do
  subject(:component) { component_class.new(current_user: current_user) }

  let(:current_user) { build_user(name: 'Admin Doe', administrator: true) }
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
    it 'is true for administrators' do
      expect(component.send(:user_is_admin?)).to be(true)
    end

    context 'when current_user is not an administrator' do
      let(:current_user) { build_user(name: 'Carer Person', administrator: false) }

      it 'is false' do
        expect(component.send(:user_is_admin?)).to be(false)
      end
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

  describe '#current_user_initials' do
    it 'returns initials from the user name' do
      expect(component.send(:current_user_initials)).to eq('AD')
    end

    context 'when the user has no name' do
      let(:current_user) { build_user(name: nil, administrator: false) }

      it 'falls back to U' do
        expect(component.send(:current_user_initials)).to eq('U')
      end
    end
  end

  def build_user(name:, administrator:)
    Class.new do
      attr_reader :name

      def initialize(name:, administrator:)
        @name = name
        @administrator = administrator
      end

      def administrator?
        @administrator
      end
    end.new(name: name, administrator: administrator)
  end
end
