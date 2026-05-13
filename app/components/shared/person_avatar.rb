# frozen_string_literal: true

require 'digest/md5'

module Components
  module Shared
    class PersonAvatar < Components::Base
      SIZES = {
        xs: { class: 'h-6 w-6 text-[0.625rem]', pixels: 24 },
        sm: { class: 'h-8 w-8 text-xs', pixels: 32 },
        md: { class: 'h-10 w-10 text-sm', pixels: 40 },
        lg: { class: 'h-14 w-14 text-lg', pixels: 56 },
        xl: { class: 'h-24 w-24 text-3xl', pixels: 96 }
      }.freeze

      attr_reader :person, :size, :gravatar, :attrs

      def self.initials_for(person)
        person.name.to_s.split.filter_map { |part| part[0] }.join.upcase.presence || '?'
      end

      def initialize(person:, size: :md, gravatar: true, **attrs)
        @person = person
        @size = size.to_sym
        @gravatar = gravatar
        @attrs = merged_attrs(attrs)
        super()
      end

      def view_template
        span(**attrs) do
          render_fallback
          render_image if avatar_url.present?
        end
      end

      private

      def merged_attrs(user_attrs)
        base_attrs = default_attrs
        base_attrs[:class] = [base_attrs[:class], user_attrs[:class]].compact.join(' ')
        base_attrs[:class] = RubyUI::Base::TAILWIND_MERGER.merge(base_attrs[:class])
        base_attrs[:data] = base_attrs.fetch(:data, {}).merge(user_attrs.fetch(:data, {}))
        base_attrs[:aria] = base_attrs.fetch(:aria, {}).merge(user_attrs.fetch(:aria, {}))
        base_attrs.merge(user_attrs.except(:class, :data, :aria))
      end

      def default_attrs
        {
          class: [
            'relative inline-flex shrink-0 overflow-hidden rounded-shape-full bg-secondary-container',
            'text-on-secondary-container shadow-inner ring-1 ring-outline-variant/40',
            size_config.fetch(:class)
          ].join(' '),
          data: { testid: 'person-avatar' },
          aria: { label: person.name }
        }
      end

      def render_fallback
        span(class: 'flex h-full w-full items-center justify-center font-black tracking-normal') do
          plain self.class.initials_for(person)
        end
      end

      def render_image
        img(
          src: avatar_url,
          alt: person.name,
          loading: 'lazy',
          class: 'absolute inset-0 h-full w-full object-cover',
          data: {
            controller: 'avatar-image',
            action: 'error->avatar-image#error'
          }
        )
      end

      def avatar_url
        @avatar_url ||= uploaded_avatar_url || gravatar_url
      end

      def uploaded_avatar_url
        return unless person.avatar.attached?

        view_context.url_for(person.avatar)
      end

      def gravatar_url
        return unless gravatar
        return unless gravatar_enabled?
        return if avatar_email.blank?

        digest = Digest::MD5.hexdigest(avatar_email.downcase)
        "https://www.gravatar.com/avatar/#{digest}?d=404&s=#{size_config.fetch(:pixels)}"
      end

      def gravatar_enabled?
        person.user&.gravatar_enabled?
      end

      def avatar_email
        person.email.presence || person.user&.email_address.presence || person.account&.email
      end

      def size_config
        SIZES.fetch(size) { SIZES.fetch(:md) }
      end
    end
  end
end
