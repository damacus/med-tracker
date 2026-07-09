# frozen_string_literal: true

module Components
  module Layouts
    module NavigationItems
      private

      def primary_navigation_items
        [
          navigation_item(t('layouts.sidebar.dashboard'), household_navigation_path(:dashboard_path), Icons::Home),
          navigation_item(t('layouts.sidebar.inventory'), household_navigation_path(:medications_path), Icons::Inventory),
          navigation_item(t('layouts.sidebar.locations'), household_navigation_path(:locations_path), Icons::Home),
          navigation_item(t('layouts.sidebar.people'), household_navigation_path(:people_path), Icons::Users),
          navigation_item(t('layouts.sidebar.finder'), household_navigation_path(:medication_finder_path), Icons::Search),
          navigation_item(t('layouts.sidebar.medicine_reviews'),
                          household_navigation_path(:medication_review_prompts_path), Icons::FileText),
          navigation_item(t('layouts.sidebar.reports'), household_navigation_path(:reports_path), Icons::AlertCircle)
        ]
      end

      def admin_navigation_items
        return [] unless user_is_admin?

        [navigation_item(t('layouts.sidebar.administration'), admin_root_path, Icons::Settings)]
      end

      def profile_navigation_item
        navigation_item(t('layouts.profile_menu.profile'), household_navigation_path(:profile_path), Icons::User)
      end

      def active_navigation_path?(path)
        current_path = view_context.request.path
        dashboard_path = household_navigation_path(:dashboard_path)

        return true if current_path == path
        return true if path == root_path && current_path == dashboard_path
        return true if path != root_path && current_path.start_with?(path)

        false
      end

      def navigation_item(label, path, icon)
        { label:, path:, icon: }
      end

      def household_navigation_path(helper_name)
        return root_path if household_route_options.blank?

        public_send(helper_name, household_route_options)
      end

      def household_route_options
        @household_route_options ||= begin
          slug = household_slug
          slug ? { household_slug: slug } : {}
        end
      end

      def household_slug
        Current.household&.slug || account_household_slug || default_household_slug
      end

      def account_household_slug
        navigation_account&.first_active_household&.slug
      end

      def navigation_account
        return unless current_user.respond_to?(:person)

        current_user&.person&.account
      end

      def default_household_slug
        options = view_context.default_url_options
        options[:household_slug] || options['household_slug']
      end
    end
  end
end
