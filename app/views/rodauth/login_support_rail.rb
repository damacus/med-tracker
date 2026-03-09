# frozen_string_literal: true

module Views
  module Rodauth
    module LoginSupportRail
      private

      def render_security_panel
        aside(class: 'relative flex flex-col justify-between border-t border-black/8 bg-[linear-gradient(180deg,rgba(246,241,233,0.95),rgba(236,242,238,0.95))] p-8 lg:w-56 lg:border-l lg:border-t-0') do
          div(class: 'space-y-8') do
            div(class: 'space-y-3') do
              p(class: 'text-[0.68rem] font-black uppercase tracking-[0.3em] text-zinc-500') { 'Why this feels different' }
              p(class: 'text-sm leading-7 text-zinc-700') do
                plain 'The screen is structured so the form stays quiet while trust signals and alternate routes are obvious at a glance.'
              end
            end

            div(class: 'space-y-4') do
              security_detail(Components::Icons::Fingerprint, 'Passkeys on the front door', 'Biometric sign-in is promoted alongside password login, not hidden behind setup flows.')
              security_detail(Components::Icons::Lock, 'Security without ceremony', 'Fallback routes stay available, but the visual priority remains clear and calm.')
              security_detail(Components::Icons::Sparkles, 'Responsive and deliberate', 'The composition compresses cleanly on mobile while preserving hierarchy.')
            end
          end

          div(class: 'mt-8 rounded-[1.8rem] border border-black/10 bg-white/80 p-5 shadow-[inset_0_1px_0_rgba(255,255,255,0.85)]') do
            p(class: 'text-[0.68rem] font-black uppercase tracking-[0.28em] text-zinc-500') { 'Support links' }
            div(class: 'mt-4 space-y-3 text-sm text-zinc-700') do
              p { 'Password reset and verification resend remain directly accessible below.' }
            end
          end
        end
      end

      def security_detail(icon_class, title, body)
        div(class: 'rounded-[1.6rem] border border-black/8 bg-white/70 p-4 shadow-[0_12px_30px_-26px_rgba(15,23,42,0.5)]') do
          div(class: 'flex items-start gap-4') do
            div(class: 'inline-flex h-11 w-11 items-center justify-center rounded-2xl bg-zinc-950 text-white shadow-sm') do
              render icon_class.new(size: 18)
            end
            div(class: 'space-y-1.5') do
              p(class: 'text-sm font-black text-zinc-950') { title }
              p(class: 'text-sm leading-6 text-zinc-600') { body }
            end
          end
        end
      end

      def render_signup_prompt
        div(class: 'mt-10 flex flex-col gap-4 border-t border-black/10 pt-6 text-center sm:text-left') do
          render_create_account_link unless invite_only?
          render_resend_verification_link
        end
      end

      def render_create_account_link
        Text(size: '2', weight: 'medium', class: 'text-zinc-600') do
          plain "#{t('sessions.login.need_account')} "
          render RubyUI::Link.new(href: view_context.rodauth.create_account_path, variant: :link,
                                  class: 'p-0 h-auto font-black text-zinc-950 hover:text-amber-700 hover:underline') do
            t('sessions.login.create_account')
          end
        end
      end

      def render_resend_verification_link
        div do
          render RubyUI::Link.new(href: view_context.rodauth.verify_account_resend_path, variant: :link,
                                  class: 'text-xs font-black uppercase tracking-[0.22em] text-zinc-500 hover:text-zinc-950') do
            t('sessions.login.resend_verification')
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        div(id: 'login-flash', class: 'mb-8') do
          render RubyUI::Alert.new(variant: flash_variant,
                                   class: 'rounded-[1.6rem] border-none bg-white/80 shadow-[0_16px_35px_-28px_rgba(15,23,42,0.6)] text-center') do
            plain(flash_message)
          end
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end

      def render_oidc_icon
        render Components::Icons::Globe.new(size: 20, class: 'mr-3')
      end
    end
  end
end
