"""
Middleware for desktop application authentication
"""
from django.contrib.auth.models import User
from django.utils.deprecation import MiddlewareMixin
from django.middleware.csrf import CsrfViewMiddleware
from django.conf import settings
import logging

logger = logging.getLogger(__name__)


class DesktopUserMiddleware(MiddlewareMixin):
    """
    Automatically assign desktop user to all requests.
    This simplifies authentication for single-user desktop applications.
    """
    
    def __init__(self, get_response):
        super().__init__(get_response)
        self.get_response = get_response
        
        # Create desktop user once on startup
        self.desktop_user, created = User.objects.get_or_create(
            username='desktop_user',
            defaults={
                'email': 'user@desktop.local',
                'first_name': 'Desktop',
                'last_name': 'User',
                'is_active': True
            }
        )
        
        if created:
            logger.info("Created desktop user for save editor")
        else:
            logger.info("Using existing desktop user for save editor")
    
    def process_request(self, request):
        """Auto-assign desktop user to every request if in desktop mode"""
        # Only activate in desktop mode
        if not settings.DESKTOP_MODE:
            logger.debug(f"Desktop mode disabled, skipping auto-auth for: {request.path}")
            return None
            
        # Skip for admin and auth URLs
        if request.path.startswith('/admin/') or request.path.startswith('/auth/'):
            logger.debug(f"Skipping auto-auth for admin/auth URL: {request.path}")
            return None
            
        # Always assign desktop user in desktop mode
        request.user = self.desktop_user
        request._cached_user = self.desktop_user
        
        return None