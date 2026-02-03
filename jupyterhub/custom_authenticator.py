"""
Custom JupyterHub Authenticator for UJ AI Club
Authenticates users via JWT tokens from the main application
"""

import jwt
import os
from jupyterhub.auth import Authenticator
from traitlets import Unicode, Bool


class JWTAuthenticator(Authenticator):
    """
    Custom authenticator that validates JWT tokens from the main UJ AI Club application.
    Enables SSO so users don't need to log in again when accessing JupyterHub.
    """
    
    jwt_secret = Unicode(
        config=True,
        help="The secret key used to decode JWT tokens (must match the main app's JWT_SECRET)"
    )
    
    auto_login = Bool(
        True,
        config=True,
        help="Automatically login users with valid tokens"
    )
    
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.jwt_secret = os.environ.get('JWT_SECRET', '')
        if not self.jwt_secret:
            self.log.warning("JWT_SECRET environment variable not set - authentication will fail")
    
    async def authenticate(self, handler, data):
        """
        Authenticate a user based on JWT token.
        Token can be passed via:
        1. Query parameter: ?token=<jwt_token>
        2. Authorization header: Bearer <jwt_token>
        """
        token = None
        
        # Try to get token from query parameter
        token = handler.get_argument('token', None)
        
        # If not in query, try Authorization header
        if not token:
            auth_header = handler.request.headers.get('Authorization', '')
            if auth_header.startswith('Bearer '):
                token = auth_header[7:]
        
        if not token:
            self.log.warning("No JWT token provided")
            return None
        
        try:
            # Decode and verify the JWT token
            payload = jwt.decode(
                token,
                self.jwt_secret,
                algorithms=['HS256']
            )
            
            # Verify this is a JupyterHub SSO token
            if payload.get('purpose') != 'jupyterhub_sso':
                self.log.warning("Invalid token purpose")
                return None
            
            username = payload.get('username')
            if not username:
                self.log.warning("No username in token")
                return None
            
            self.log.info(f"Successfully authenticated user: {username}")
            
            # Return user data
            return {
                'name': username,
                'admin': False,
                'auth_state': {
                    'user_id': payload.get('sub'),
                    'token': token
                }
            }
            
        except jwt.ExpiredSignatureError:
            self.log.warning("JWT token has expired")
            return None
        except jwt.InvalidTokenError as e:
            self.log.warning(f"Invalid JWT token: {e}")
            return None
        except Exception as e:
            self.log.error(f"Authentication error: {e}")
            return None
    
    def get_handlers(self, app):
        """Return custom handlers for token-based login."""
        return [
            (r'/login', TokenLoginHandler),
        ]
    
    async def pre_spawn_start(self, user, spawner):
        """
        Called before the user's server is spawned.
        Can be used to set up user-specific configurations.
        """
        auth_state = await user.get_auth_state()
        if auth_state:
            # Pass user_id to the spawner for environment setup
            spawner.environment['AICLUB_USER_ID'] = auth_state.get('user_id', '')


from jupyterhub.handlers import BaseHandler
from tornado import web


class TokenLoginHandler(BaseHandler):
    """Handler for token-based login."""
    
    async def get(self):
        """Handle GET request with token in query parameter."""
        token = self.get_argument('token', None)
        next_url = self.get_argument('next', '/')
        
        if not token:
            # Redirect to main app login
            main_app_url = os.environ.get('MAIN_APP_URL', 'https://aiclub-uj.com')
            self.redirect(f"{main_app_url}/login?redirect=jupyterhub")
            return
        
        # Authenticate with the token
        user = await self.login_user()
        
        if user:
            self.redirect(next_url)
        else:
            self.set_status(401)
            self.write("Authentication failed. Please log in through the main application.")


class AdminJWTAuthenticator(JWTAuthenticator):
    """
    Extended authenticator that also supports admin users.
    Admin status is determined by the main application.
    """
    
    admin_users_api = Unicode(
        config=True,
        help="API endpoint to check if a user is an admin"
    )
    
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.admin_users_api = os.environ.get(
            'ADMIN_USERS_API',
            'https://api.aiclub-uj.com/users/profile'
        )
    
    async def authenticate(self, handler, data):
        """Authenticate and check admin status."""
        result = await super().authenticate(handler, data)
        
        if result and isinstance(result, dict):
            # Could add admin check here by calling the main app API
            # For now, no JupyterHub admins from regular users
            result['admin'] = False
        
        return result
