# frozen_string_literal: true

module Components
  module Layouts
    module NavigationItems
      private

      def primary_navigation_items
        [
          navigation_item(t("layouts.sidebar.dashboard"), root_path, Icons::Home),
          navigation_item(t("layouts.sidebar.inventory"), medications_path, Icons::Pill),
          navigation_item(t("layouts.sidebar.locations"), locations_path, Icons::Home),
          navigation_item(t("layouts.sidebar.people"), people_path, Icons::Users),
          navigation_item(t("layouts.sidebar.finder"), medication_finder_path, Icons::Search),
          navigation_item(t("layouts.sidebar.reports"), reports_path, Icons::AlertCircle)
        ]
      end

      def admin_navigation_items
        return [] unless user_is_admin?

        [navigation_item(t("layouts.sidebar.administration"), admin_root_path, Icons::Settings)]
      end

      def profile_navigation_item
        navigation_item(t("layouts.profile_menu.profile"), profile_path, Icons::User)
      end

      def active_navigation_path?(path)
        current_path = view_context.request.path

        return true if current_path == path
        return true if path == root_path && current_path == dashboard_path
        return true if path != root_path && current_path.start_with?(path)

        false
      end

      def navigation_item(label, path, icon)
        {label:, path:, icon:}
      end
    end
  end
end
