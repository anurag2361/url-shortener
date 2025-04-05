use std::future::{Ready, ready};

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    error::ErrorUnauthorized,
    http::header,
};
use futures_util::future::LocalBoxFuture;

use crate::utils::jwt::{Claims, validate_token};

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

        // Store the complete Claims object in request extensions for later use
        req.extensions_mut().insert(claims);

        Box::pin(self.service.call(req))
    }
}
