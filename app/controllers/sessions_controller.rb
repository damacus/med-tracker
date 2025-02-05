class SessionsController < ApplicationController
  def new
    # Login form
  end

  def create
    # TODO: Implement actual authentication
    flash[:notice] = "Authentication not yet implemented"
    redirect_to root_path
  end

  def destroy
    # TODO: Implement logout
    flash[:notice] = "Logged out successfully"
    redirect_to root_path
  end
end
