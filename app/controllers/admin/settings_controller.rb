# frozen_string_literal: true

module Admin
  class SettingsController < BaseController
    def show
      @settings = AppSettings.instance
      authorize @settings
      render Components::Admin::Settings::ShowView.new(settings: @settings)
    end

    def update
      @settings = AppSettings.instance
      authorize @settings

      respond_to do |format|
        if @settings.update(settings_params)
          format.html { redirect_to admin_settings_path, notice: t('admin.settings.updated') }
          format.turbo_stream do
            flash.now[:notice] = t('admin.settings.updated')
            render turbo_stream: [
              turbo_stream.replace('admin_settings', Components::Admin::Settings::ShowView.new(settings: @settings)),
              turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
            ]
          end
        else
          format.html do
            render Components::Admin::Settings::ShowView.new(settings: @settings), status: :unprocessable_content
          end
        end
      end
    end

    private

    def settings_params
      params.expect(app_settings: [
                      :invite_only,
                      :medicine_lookup_base_url,
                      :medicine_lookup_token_url,
                      { medicine_lookup_source_priority: [] }
                    ])
    end
  end
end
