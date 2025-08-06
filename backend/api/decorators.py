"""
Custom decorators for the API
"""
from functools import wraps
from django.conf import settings
from rest_framework.decorators import permission_classes
from rest_framework.permissions import IsAuthenticated


def desktop_or_authenticated(view_func):
    """
    Decorator that requires authentication only if not in desktop mode.
    In desktop mode, the middleware handles authentication automatically.
    
    This decorator works at request time to properly handle desktop mode.
    """
    @wraps(view_func)
    def wrapped_view(request, *args, **kwargs):
        import logging
        logger = logging.getLogger(__name__)
        logger.info(f"desktop_or_authenticated decorator called for {view_func.__name__}")
        logger.info(f"DESKTOP_MODE: {settings.DESKTOP_MODE}, user: {request.user}, authenticated: {request.user.is_authenticated}")
        
        # Check desktop mode at request time, not import time
        if settings.DESKTOP_MODE:
            # In desktop mode, bypass DRF permission system entirely
            # The middleware should have already set request.user
            logger.info("Desktop mode - bypassing authentication")
            return view_func(request, *args, **kwargs)
        else:
            # In web mode, check if user is authenticated
            if not request.user.is_authenticated:
                logger.warning("Web mode - user not authenticated")
                from rest_framework.response import Response
                from rest_framework import status
                return Response(
                    {'error': {'message': 'Authentication required', 'code': 'AUTHENTICATION_REQUIRED'}},
                    status=status.HTTP_401_UNAUTHORIZED
                )
            logger.info("Web mode - user authenticated")
            return view_func(request, *args, **kwargs)
            
    # Important: Apply permission_classes([AllowAny]) to bypass DRF's permission system
    # We handle permissions manually in the wrapper above
    wrapped_view = permission_classes([])(wrapped_view)
    return wrapped_view