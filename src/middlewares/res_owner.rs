use std::future::{Ready, ready};

use crate::utils::jwt::Claims;
use actix_web::error::ErrorForbidden;
use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;

pub struct ResourceOwnership {
    pub param_name: String, // Name of the URL parameter that contains the user ID
}

impl<S, B> Transform<S, ServiceRequest> for ResourceOwnership
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = ResourceOwnershipMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ResourceOwnershipMiddleware {
            service,
            param_name: self.param_name.clone(),
        }))
    }
}

pub struct ResourceOwnershipMiddleware<S> {
    service: S,
    param_name: String,
}

impl<S, B> Service<ServiceRequest> for ResourceOwnershipMiddleware<S>
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
        // Get the current user ID from JWT claims
        let current_user_id = match req.extensions().get::<Claims>() {
            Some(claims) => claims.user_id.clone(),
            None => {
                return Box::pin(async move { Err(ErrorForbidden("User not authenticated")) });
            }
        };

        // Extract the resource owner ID from the URL path
        let path = req.match_info();
        let resource_owner_id = match path.get(&self.param_name) {
            Some(id) => id.to_string(),
            None => {
                // If no user_id parameter, continue (might be a collection endpoint)
                return Box::pin(self.service.call(req));
            }
        };

        // Check if the current user is accessing their own resources or is an admin
        if current_user_id != resource_owner_id {
            return Box::pin(async move {
                Err(ErrorForbidden(
                    "Access denied: You can only access your own resources",
                ))
            });
        }

        // User is accessing their own resources, proceed
        Box::pin(self.service.call(req))
    }
}
