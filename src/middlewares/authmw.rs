use std::future::{Ready, ready};

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    error::ErrorUnauthorized,
    http::header,
};
use futures_util::future::LocalBoxFuture;

use crate::models::role::Role;
use crate::utils::jwt::validate_token;

pub struct JwtAuth;

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware { service }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip auth for login and redirect routes
        let path = req.path();
        if path.starts_with("/api/auth/login")
            || path.starts_with("/r/")
            || path.starts_with("/api/health/check")
        {
            return Box::pin(self.service.call(req));
        }

        // Get token from Authorization header
        let auth_header = req.headers().get(header::AUTHORIZATION);
        let auth_header = match auth_header {
            Some(header) => header,
            None => {
                return Box::pin(async move { Err(ErrorUnauthorized("No authorization header")) });
            }
        };

        // Extract the token from the Authorization header
        let auth_header_str = match auth_header.to_str() {
            Ok(header_str) => header_str,
            Err(_) => {
                return Box::pin(
                    async move { Err(ErrorUnauthorized("Invalid authorization header")) },
                );
            }
        };

        // Check if the header starts with "Bearer "
        if !auth_header_str.starts_with("Bearer ") {
            return Box::pin(async move { Err(ErrorUnauthorized("Invalid authorization format")) });
        }

        // Extract the token
        let token = &auth_header_str[7..];

        // Validate the token
        let claims = match validate_token(token) {
            Ok(claims) => claims,
            Err(_) => {
                return Box::pin(async move { Err(ErrorUnauthorized("Invalid token")) });
            }
        };

        // Store claims in request extensions for later use
        req.extensions_mut().insert(claims.sub.clone());
        req.extensions_mut().insert(claims.roles);

        Box::pin(self.service.call(req))
    }
}

// Extra middleware for specific role checks
pub struct RequireRoles(pub Vec<Role>);

impl<S, B> Transform<S, ServiceRequest> for RequireRoles
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequireRolesMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequireRolesMiddleware {
            service,
            required_roles: self.0.clone(),
        }))
    }
}

pub struct RequireRolesMiddleware<S> {
    service: S,
    required_roles: Vec<Role>,
}

impl<S, B> Service<ServiceRequest> for RequireRolesMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Check if user roles are present in request extensions (set by JwtAuthMiddleware)
        let user_roles = req.extensions().get::<Vec<Role>>().cloned();

        let user_roles = match user_roles {
            Some(roles) => roles,
            None => {
                return Box::pin(async move { Err(ErrorUnauthorized("Authentication required")) });
            }
        };

        // Check if user has any of the required roles
        let has_required_role = user_roles.iter().any(|role| {
            if role == &Role::SuperUser {
                return true; // SuperUser has all permissions
            }
            self.required_roles.contains(role)
        });

        if !has_required_role {
            return Box::pin(async move { Err(ErrorUnauthorized("Insufficient permissions")) });
        }

        Box::pin(self.service.call(req))
    }
}
