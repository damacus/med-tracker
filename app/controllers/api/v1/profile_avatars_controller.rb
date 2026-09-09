module Api
  module V1
    class ProfileAvatarsController < ProfilesController
      rescue_from Profiles::Avatar::StorageError do
        render_api_error(code: 'avatar_unavailable', message: 'Avatar is temporarily unavailable',
                         status: :service_unavailable)
      end

      def show
        raise ActiveRecord::RecordNotFound unless profile_person.avatar.attached?

        blob = profile_person.avatar.blob
        send_data avatar_service.download, filename: blob.filename.to_s, type: blob.content_type, disposition: 'inline'
      end

      def update
        avatar_service.upload(params.expect(:avatar))
        render_profile
      end

      def destroy
        avatar_service.remove
        head :no_content
      end

      private

      def with_api_idempotency(&action)
        response.set_header('Cache-Control', 'no-store')
        authorize profile_person, action_name == 'show' ? :show? : :update?
        action.call
      end

      def avatar_service
        Profiles::Avatar.new(person: profile_person, authorization: pundit_user)
      end
    end
  end
end
