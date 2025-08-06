"""
Debug middleware to trace 403 errors
"""
import logging
from django.utils.deprecation import MiddlewareMixin

logger = logging.getLogger(__name__)


class DebugResponseMiddleware(MiddlewareMixin):
    """Log all responses, especially 403s"""
    
    def process_request(self, request):
        logger.info(f"=== DebugResponseMiddleware process_request ===")
        logger.info(f"User at start: {request.user}")
        logger.info(f"User type: {type(request.user)}")
        logger.info(f"Authenticated: {request.user.is_authenticated}")
        return None
    
    def process_response(self, request, response):
        if response.status_code == 403:
            logger.warning(f"=== 403 FORBIDDEN RESPONSE ===")
            logger.warning(f"Request path: {request.path}")
            logger.warning(f"Request method: {request.method}")
            logger.warning(f"Request user: {request.user}")
            logger.warning(f"User authenticated: {request.user.is_authenticated}")
            logger.warning(f"Response status: {response.status_code}")
            logger.warning(f"Response content: {response.content[:500]}")  # First 500 chars
            logger.warning(f"Response headers: {dict(response.headers)}")
            logger.warning(f"=== END 403 DEBUG ===")
        
        return response
    
    def process_exception(self, request, exception):
        logger.error(f"=== EXCEPTION IN REQUEST ===")
        logger.error(f"Request: {request.method} {request.path}")
        logger.error(f"Exception type: {type(exception)}")
        logger.error(f"Exception: {exception}")
        logger.error(f"=== END EXCEPTION ===")
        return None