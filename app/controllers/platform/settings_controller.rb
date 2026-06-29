# frozen_string_literal: true

module Platform
  class SettingsController < BaseController
    def show
      @settings = AppSettings.instance
      authorize @settings
      render Components::Platform::Settings::ShowView.new(settings: @settings)
    end

    def update
      @settings = AppSettings.instance
      authorize @settings

      if @settings.update(settings_params)
        redirect_to platform_settings_path, notice: t('admin.settings.updated')
      else
        render Components::Platform::Settings::ShowView.new(settings: @settings), status: :unprocessable_content
      end
    end

    private

    def settings_params
      params.expect(app_settings: [:invite_only])
    end
  end
end
